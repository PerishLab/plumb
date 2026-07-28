const USER_AGENT = "plumb-release-stable/1.0";
const RETRY_MS = 15_000;
const STABLE = /^(\d+)\.(\d+)\.(\d+)$/;
const TAGGED = /^v?(\d+\.\d+\.\d+)$/;

function fail(message: string): never {
	console.error(`[release-stable] ${message}`);
	Deno.exit(1);
}

function tuple(value: string): [number, number, number] {
	const match = STABLE.exec(value);
	if (!match) {
		fail(`expected stable x.y.z version, got ${value}`);
	}
	return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function order(left: string, right: string): number {
	const a = tuple(left);
	const b = tuple(right);
	for (let index = 0; index < 3; index += 1) {
		if (a[index] !== b[index]) {
			return a[index] < b[index] ? -1 : 1;
		}
	}
	return 0;
}

async function cargoVersion(): Promise<string> {
	const text = await Deno.readTextFile("Cargo.toml");
	const match = /^version = "([^"]+)"$/m.exec(text);
	if (!match) {
		fail("missing workspace version in Cargo.toml");
	}
	tuple(match[1]);
	return match[1];
}

function parseStable(value: string, source: string): string {
	const match = TAGGED.exec(value);
	if (!match) {
		fail(`${source} must look like vX.Y.Z, got ${value}`);
	}
	return match[1];
}

async function output(name: string, value: string): Promise<void> {
	const path = Deno.env.get("GITHUB_OUTPUT");
	if (path) {
		await Deno.writeTextFile(path, `${name}=${value}\n`, { append: true });
	}
}

async function attempt(url: string): Promise<[string | null, number | null]> {
	try {
		const response = await fetch(url, {
			headers: { "Cache-Control": "no-cache", "User-Agent": USER_AGENT },
		});
		if (response.ok) {
			return [await response.text(), null];
		}
		await response.body?.cancel();
		return [null, response.status];
	} catch (error) {
		fail(`failed to fetch R2 stable metadata: ${error}`);
	}
}

async function optional(url: string): Promise<string | null> {
	let [text, status] = await attempt(url);
	if (text !== null) {
		return text;
	}
	if (status === 403) {
		fail(
			"R2 stable metadata returned 403; refusing to treat permission failure as absence",
		);
	}
	if (status === 404) {
		console.log(`[release-stable] metadata 404; retrying after ${RETRY_MS}ms`);
		await new Promise((resolve) => setTimeout(resolve, RETRY_MS));
		[text, status] = await attempt(url);
		if (text !== null) {
			return text;
		}
		if (status === 403) {
			fail("R2 stable metadata returned 403 on retry");
		}
		if (status === 404) {
			return null;
		}
	}
	fail(`failed to fetch R2 stable metadata: HTTP ${status}`);
}

function prior(metadata: Record<string, unknown>): string {
	const value =
		metadata.stableVersion || metadata.releaseVersion || metadata.baseVersion;
	if (typeof value !== "string" || !value) {
		fail("R2 stable metadata has no usable stable version");
	}
	return parseStable(value, "R2 stable metadata");
}

async function proven(cargo: string): Promise<string> {
	const publicUrl = (Deno.env.get("PLUMB_RELEASES_PUBLIC_URL") ?? "").replace(
		/\/+$/,
		"",
	);
	const url =
		Deno.env.get("PLUMB_BETA_METADATA_URL") ||
		(publicUrl ? `${publicUrl}/beta/latest/metadata.json` : "");
	if (!url) {
		fail("PLUMB_RELEASES_PUBLIC_URL is required");
	}
	console.log(`[release-stable] beta proof url: ${url}`);
	const text = await optional(url);
	if (text === null) {
		fail(`stable ${cargo} has no beta proof`);
	}
	let metadata: Record<string, unknown>;
	try {
		metadata = JSON.parse(text) as Record<string, unknown>;
	} catch (error) {
		fail(`R2 beta metadata is invalid JSON: ${error}`);
	}
	const raw = metadata.betaVersion || metadata.releaseVersion;
	const match =
		typeof raw === "string"
			? /^v?(\d+\.\d+\.\d+)-beta\.([1-9][0-9]*)$/.exec(raw)
			: null;
	if (!match || match[1] !== cargo || metadata.baseVersion !== cargo) {
		fail(`latest beta does not prove stable base ${cargo}`);
	}
	const commit = (metadata.ci as Record<string, unknown> | undefined)?.commit;
	const expected = (Deno.env.get("GITHUB_SHA") ?? "").trim();
	if (!expected) {
		fail("GITHUB_SHA is required to prove stable promotion");
	}
	if (commit !== expected) {
		fail(`latest beta ${raw} came from ${String(commit)}, not ${expected}`);
	}
	return raw as string;
}

async function next(cargo: string): Promise<[string, string, string]> {
	const publicUrl = (Deno.env.get("PLUMB_RELEASES_PUBLIC_URL") ?? "").replace(
		/\/+$/,
		"",
	);
	const url =
		Deno.env.get("PLUMB_STABLE_METADATA_URL") ||
		(publicUrl ? `${publicUrl}/stable/latest/metadata.json` : "");
	if (!url) {
		fail("PLUMB_RELEASES_PUBLIC_URL is required");
	}
	console.log(`[release-stable] metadata url: ${url}`);
	const text = await optional(url);
	if (text === null) {
		return [cargo, `v${cargo}`, "missing R2 stable metadata"];
	}
	let metadata: unknown;
	try {
		metadata = JSON.parse(text);
	} catch (error) {
		fail(`R2 stable metadata is invalid JSON: ${error}`);
	}
	if (typeof metadata !== "object" || metadata === null) {
		fail("R2 stable metadata must be an object");
	}
	const before = prior(metadata as Record<string, unknown>);
	const ranked = order(cargo, before);
	if (ranked < 0) {
		fail(`Cargo version ${cargo} regressed below prior stable ${before}`);
	}
	if (ranked === 0) {
		fail(
			`Cargo version ${cargo} matches prior stable; bump Cargo.toml before re-running`,
		);
	}
	return [cargo, `v${cargo}`, `R2 stable metadata v${before}`];
}

const cargo = await cargoVersion();
const beta = await proven(cargo);
const override = (Deno.env.get("STABLE_VERSION_OVERRIDE") ?? "").trim();
let base: string;
let version: string;
let source: string;
if (override) {
	base = parseStable(override, "STABLE_VERSION_OVERRIDE");
	if (base !== cargo) {
		fail(`override base ${base} does not match Cargo version ${cargo}`);
	}
	version = `v${base}`;
	source = `workflow override after ${beta}`;
} else {
	[base, version, source] = await next(cargo);
	source = `${source}; proved by ${beta}`;
}
console.log(`[release-stable] ${version} from ${source}`);
await output("base_version", base);
await output("release_version", version);
await output("state_source", source);
