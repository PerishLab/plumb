const REGISTRY = "perish";
const PACKAGES = ["plumb-macro", "plumb"] as const;
const VERSION = /^(\d+)\.(\d+)\.(\d+)(?:-([a-z][a-z0-9-]*)\.([1-9][0-9]*))?$/;
const CHANNEL = /^[a-z][a-z0-9-]*$/;
const POLL_MS = 5_000;
const POLL_ATTEMPTS = 12;

type PackageName = (typeof PACKAGES)[number];

type Identity = {
	version: string;
	core: [number, number, number];
	channel?: string;
	number?: number;
};

type RegistryEntry = {
	name: string;
	vers: string;
	cksum: string;
	yanked?: boolean;
};

function fail(message: string): never {
	console.error(`[release-cargo] ${message}`);
	Deno.exit(1);
}

function parseVersion(raw: string, source: string): Identity {
	const version = raw.trim().replace(/^v/, "");
	const match = VERSION.exec(version);
	if (!match) {
		fail(`${source} must look like X.Y.Z or X.Y.Z-<channel>.N, got ${raw}`);
	}
	return {
		version,
		core: [Number(match[1]), Number(match[2]), Number(match[3])],
		channel: match[4],
		number: match[5] ? Number(match[5]) : undefined,
	};
}

function compare(left: Identity, right: Identity): number {
	for (let index = 0; index < left.core.length; index += 1) {
		if (left.core[index] !== right.core[index]) {
			return left.core[index] < right.core[index] ? -1 : 1;
		}
	}
	if (left.channel === undefined || right.channel === undefined) {
		if (left.channel === right.channel) return 0;
		return left.channel === undefined ? 1 : -1;
	}
	if (left.channel !== right.channel) {
		return left.channel < right.channel ? -1 : 1;
	}
	return (left.number ?? 0) - (right.number ?? 0);
}

function releaseIdentity(): Identity {
	const channel = (Deno.env.get("RELEASE_CHANNEL") ?? "").trim();
	const raw = (Deno.env.get("RELEASE_VERSION") ?? "").trim();
	if (!CHANNEL.test(channel)) {
		fail(`invalid RELEASE_CHANNEL: ${channel}`);
	}
	if (!raw) {
		fail("RELEASE_VERSION is required");
	}
	const identity = parseVersion(raw, "RELEASE_VERSION");
	if (channel === "stable" && identity.channel !== undefined) {
		fail(`stable cannot publish prerelease ${identity.version}`);
	}
	if (channel !== "stable" && identity.channel !== channel) {
		fail(`version ${identity.version} does not belong to channel ${channel}`);
	}
	return identity;
}

async function text(path: string): Promise<string> {
	try {
		return await Deno.readTextFile(path);
	} catch (error) {
		fail(`cannot read ${path}: ${error}`);
	}
}

async function write(path: string, value: string): Promise<void> {
	await Deno.writeTextFile(path, value);
}

function replaceOnce(
	source: string,
	pattern: RegExp,
	replacement: string,
	path: string,
): string {
	const matches = [
		...source.matchAll(
			new RegExp(
				pattern.source,
				pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`,
			),
		),
	];
	if (matches.length !== 1) {
		fail(`expected one stamp target in ${path}, found ${matches.length}`);
	}
	return source.replace(pattern, replacement);
}

async function stamp(identity: Identity): Promise<void> {
	const rootPath = "Cargo.toml";
	const root = await text(rootPath);
	const held = /^version = "([^"]+)"$/m.exec(root)?.[1];
	if (!held) {
		fail("missing workspace version in Cargo.toml");
	}
	const base = parseVersion(held, "workspace version");
	if (
		base.channel !== undefined ||
		base.core.join(".") !== identity.core.join(".")
	) {
		fail(
			`workspace version ${held} is not release base ${identity.core.join(
				".",
			)}`,
		);
	}
	await write(
		rootPath,
		replaceOnce(
			root,
			/^version = "[^"]+"$/m,
			`version = "${identity.version}"`,
			rootPath,
		),
	);

	const libPath = "crates/lib/Cargo.toml";
	const lib = await text(libPath);
	await write(
		libPath,
		replaceOnce(
			lib,
			/^plumb-macro = \{ path = "\.\.\/macro", version = "[^"]+", registry = "perish" \}$/m,
			`plumb-macro = { path = "../macro", version = "=${identity.version}", registry = "perish" }`,
			libPath,
		),
	);
	console.log(`[release-cargo] stamped ${identity.version}`);
}

async function command(
	name: string,
	args: string[],
	options: { capture?: boolean } = {},
): Promise<string> {
	console.log(`[release-cargo] ${name} ${args.join(" ")}`);
	const output = await new Deno.Command(name, {
		args,
		env: { CARGO_TERM_COLOR: "never" },
		stdin: "null",
		stdout: options.capture ? "piped" : "inherit",
		stderr: options.capture ? "piped" : "inherit",
	}).output();
	if (!output.success) {
		const diagnostic = options.capture
			? new TextDecoder().decode(output.stderr).trim()
			: `exit ${output.code}`;
		fail(`${name} failed: ${diagnostic}`);
	}
	return options.capture ? new TextDecoder().decode(output.stdout) : "";
}

function archivePath(name: PackageName, version: string): string {
	return `target/package/${name}-${version}.crate`;
}

async function packageCrate(
	name: PackageName,
	identity: Identity,
	verify: boolean,
): Promise<string> {
	const args = [
		"package",
		"--registry",
		REGISTRY,
		"--package",
		name,
		"--allow-dirty",
	];
	if (!verify) args.push("--no-verify");
	await command("cargo", args);
	const archive = archivePath(name, identity.version);
	const manifest = await command(
		"tar",
		["-xOf", archive, `${name}-${identity.version}/Cargo.toml`],
		{ capture: true },
	);
	const packageSection = manifestSection(manifest, "package");
	if (!packageSection.includes(`version = "${identity.version}"`)) {
		fail(`${archive} does not name package version ${identity.version}`);
	}
	if (name === "plumb") {
		const dependency = manifestSection(manifest, "dependencies.plumb-macro");
		const index = await registryIndex();
		if (
			!dependency.includes(`version = "=${identity.version}"`) ||
			!dependency.includes(`registry-index = "${index}"`)
		) {
			fail(`${archive} does not lock plumb-macro to =${identity.version}`);
		}
	}
	return archive;
}

async function listPackage(name: PackageName): Promise<void> {
	await command("cargo", [
		"package",
		"--registry",
		REGISTRY,
		"--package",
		name,
		"--allow-dirty",
		"--no-verify",
		"--list",
	]);
}

function manifestSection(manifest: string, name: string): string {
	const heading = `[${name}]\n`;
	const start = manifest.indexOf(heading);
	if (start < 0) return "";
	const body = manifest.slice(start + heading.length);
	const next = body.search(/^\[/m);
	return next < 0 ? body : body.slice(0, next);
}

async function sha256(path: string): Promise<string> {
	const digest = await crypto.subtle.digest(
		"SHA-256",
		await Deno.readFile(path),
	);
	return [...new Uint8Array(digest)]
		.map((byte) => byte.toString(16).padStart(2, "0"))
		.join("");
}

async function registryIndex(): Promise<string> {
	const config = await text(".cargo/config.toml");
	const match = /^\s*index\s*=\s*"(sparse\+[^"]+)"\s*$/m.exec(config);
	if (!match?.[1].endsWith("/")) {
		fail("missing sparse perish registry in .cargo/config.toml");
	}
	return match[1];
}

async function registryUrl(): Promise<string> {
	return (await registryIndex()).replace(/^sparse\+/, "").replace(/\/+$/, "");
}

function indexPath(name: string): string {
	const lowered = name.toLowerCase();
	if (lowered.length === 1) return `1/${lowered}`;
	if (lowered.length === 2) return `2/${lowered}`;
	if (lowered.length === 3) return `3/${lowered[0]}/${lowered}`;
	return `${lowered.slice(0, 2)}/${lowered.slice(2, 4)}/${lowered}`;
}

async function entries(name: PackageName): Promise<RegistryEntry[]> {
	const token = (Deno.env.get("CARGO_REGISTRIES_PERISH_TOKEN") ?? "").trim();
	const headers: HeadersInit = {
		"Cache-Control": "no-cache",
		"User-Agent": "plumb-release-cargo/1.0",
	};
	if (token) headers.Authorization = token;
	const response = await fetch(`${await registryUrl()}/${indexPath(name)}`, {
		headers,
	});
	if (response.status === 404) {
		await response.body?.cancel();
		return [];
	}
	if (!response.ok) {
		const status = response.status;
		await response.body?.cancel();
		fail(`registry readback for ${name} failed: HTTP ${status}`);
	}
	const body = await response.text();
	try {
		return body
			.split("\n")
			.filter(Boolean)
			.map((line) => JSON.parse(line) as RegistryEntry);
	} catch (error) {
		fail(`registry index for ${name} is malformed: ${error}`);
	}
}

function exact(
	name: PackageName,
	identity: Identity,
	held: RegistryEntry[],
): RegistryEntry | undefined {
	for (const entry of held) {
		let version: Identity;
		try {
			version = parseVersion(entry.vers, `${name} registry version`);
		} catch {
			continue;
		}
		if (compare(version, identity) > 0) {
			fail(
				`registry ${name} ${entry.vers} is ahead of target ${identity.version}`,
			);
		}
	}
	return held.find((entry) => entry.vers === identity.version);
}

function verifyChecksum(
	name: PackageName,
	identity: Identity,
	checksum: string,
	found: RegistryEntry | undefined,
): boolean {
	if (!found) return false;
	if (found.yanked) {
		fail(
			`registry ${name} ${identity.version} is yanked and cannot be republished`,
		);
	}
	if (found.cksum !== checksum) {
		fail(
			`registry ${name} ${identity.version} checksum differs: ${found.cksum} != ${checksum}`,
		);
	}
	console.log(
		`[release-cargo] ${name} ${identity.version} already published; verified`,
	);
	return true;
}

async function readback(
	name: PackageName,
	identity: Identity,
	checksum: string,
): Promise<void> {
	for (let attempt = 0; attempt < POLL_ATTEMPTS; attempt += 1) {
		const found = exact(name, identity, await entries(name));
		if (found) {
			if (found.yanked) {
				fail(`registry ${name} ${identity.version} was yanked during readback`);
			}
			if (found.cksum !== checksum) {
				fail(
					`registry ${name} ${identity.version} checksum differs: ${found.cksum} != ${checksum}`,
				);
			}
			console.log(
				`[release-cargo] readback verified ${name} ${identity.version}`,
			);
			return;
		}
		if (attempt + 1 < POLL_ATTEMPTS) {
			console.log(
				`[release-cargo] waiting for ${name} ${identity.version} readback (${
					attempt + 1
				}/${POLL_ATTEMPTS})`,
			);
			await new Promise((resolve) => setTimeout(resolve, POLL_MS));
		}
	}
	fail(`registry did not expose ${name} ${identity.version} after publish`);
}

async function dryRun(name: PackageName): Promise<void> {
	await command("cargo", [
		"publish",
		"--registry",
		REGISTRY,
		"--package",
		name,
		"--allow-dirty",
		"--dry-run",
	]);
}

async function publish(name: PackageName): Promise<void> {
	await command("cargo", [
		"publish",
		"--registry",
		REGISTRY,
		"--package",
		name,
		"--allow-dirty",
	]);
}

const mode = Deno.args[0] ?? "publish";
if (!["publish", "rehearse"].includes(mode)) {
	fail(`usage: publish.ts [publish|rehearse]`);
}
const identity = releaseIdentity();
await stamp(identity);

const macroArchive = await packageCrate("plumb-macro", identity, true);
await listPackage("plumb");
const macroChecksum = await sha256(macroArchive);

if (mode === "rehearse") {
	console.log(`[release-cargo] rehearsal true for ${identity.version}`);
	Deno.exit(0);
}

const token = (Deno.env.get("CARGO_REGISTRIES_PERISH_TOKEN") ?? "").trim();
if (!token.startsWith("Bearer ") || token.length <= "Bearer ".length) {
	fail(
		"CARGO_REGISTRIES_PERISH_TOKEN must contain Bearer plus a write:packages token",
	);
}

const macroEntry = exact("plumb-macro", identity, await entries("plumb-macro"));
const libEntry = exact("plumb", identity, await entries("plumb"));
let macroPublished = verifyChecksum(
	"plumb-macro",
	identity,
	macroChecksum,
	macroEntry,
);
if (libEntry && !macroPublished) {
	fail(
		`plumb ${identity.version} exists without plumb-macro ${identity.version}`,
	);
}

if (!macroPublished) {
	await dryRun("plumb-macro");
	if ((await sha256(macroArchive)) !== macroChecksum) {
		fail(
			`plumb-macro ${identity.version} package changed during publisher dry run`,
		);
	}
	await publish("plumb-macro");
	await readback("plumb-macro", identity, macroChecksum);
	macroPublished = true;
}

if (!macroPublished) {
	fail(`plumb-macro ${identity.version} is not readable`);
}
await dryRun("plumb");
const verifiedLib = await packageCrate("plumb", identity, true);
const verifiedLibChecksum = await sha256(verifiedLib);
const libPublished = verifyChecksum(
	"plumb",
	identity,
	verifiedLibChecksum,
	libEntry,
);
if (!libPublished) {
	await publish("plumb");
	await readback("plumb", identity, verifiedLibChecksum);
}

console.log(`[release-cargo] registry true for ${identity.version}`);
