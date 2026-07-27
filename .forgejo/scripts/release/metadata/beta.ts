const USER_AGENT = "plumb-release-beta/1.0";
const RETRY_MS = 15_000;
const STABLE = /^(\d+)\.(\d+)\.(\d+)$/;
const BETA = /^v?(\d+\.\d+\.\d+)-beta\.([1-9][0-9]*)$/;

function fail(message: string): never {
	console.error(`[release-beta] ${message}`);
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

function parseBeta(value: string, source: string): [string, number, string] {
	const match = BETA.exec(value);
	if (!match) {
		fail(`${source} must look like vX.Y.Z-beta.N, got ${value}`);
	}
	return [match[1], Number(match[2]), `v${match[1]}-beta.${match[2]}`];
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
		fail(`failed to fetch R2 beta metadata: ${error}`);
	}
}

async function optional(url: string): Promise<string | null> {
	let [text, status] = await attempt(url);
	if (text !== null) {
		return text;
	}
	if (status === 403) {
		fail(
			"R2 beta metadata returned 403; refusing to treat permission failure as absence",
		);
	}
	if (status === 404) {
		console.log(`[release-beta] metadata 404; retrying after ${RETRY_MS}ms`);
		await new Promise((resolve) => setTimeout(resolve, RETRY_MS));
		[text, status] = await attempt(url);
		if (text !== null) {
			return text;
		}
		if (status === 403) {
			fail("R2 beta metadata returned 403 on retry");
		}
		if (status === 404) {
			return null;
		}
	}
	fail(`failed to fetch R2 beta metadata: HTTP ${status}`);
}

function prior(metadata: Record<string, unknown>): [string, number, string] {
	const value = metadata.betaVersion || metadata.releaseVersion;
	if (typeof value === "string" && value) {
		return parseBeta(value, "R2 beta metadata");
	}
	if (
		typeof metadata.baseVersion === "string" &&
		typeof metadata.betaNumber === "number"
	) {
		tuple(metadata.baseVersion);
		if (metadata.betaNumber < 1) {
			fail(`R2 beta number must be >= 1, got ${metadata.betaNumber}`);
		}
		return [
			metadata.baseVersion,
			metadata.betaNumber,
			`v${metadata.baseVersion}-beta.${metadata.betaNumber}`,
		];
	}
	fail("R2 beta metadata has no usable beta version");
}

async function next(cargo: string): Promise<[string, number, string, string]> {
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
	console.log(`[release-beta] metadata url: ${url}`);
	const text = await optional(url);
	if (text === null) {
		return [cargo, 1, `v${cargo}-beta.1`, "missing R2 beta metadata"];
	}
	let metadata: unknown;
	try {
		metadata = JSON.parse(text);
	} catch (error) {
		fail(`R2 beta metadata is invalid JSON: ${error}`);
	}
	if (typeof metadata !== "object" || metadata === null) {
		fail("R2 beta metadata must be an object");
	}
	const [base, number, version] = prior(metadata as Record<string, unknown>);
	const ranked = order(cargo, base);
	if (ranked < 0) {
		fail(`Cargo version ${cargo} regressed below beta base ${base}`);
	}
	return ranked > 0
		? [cargo, 1, `v${cargo}-beta.1`, "R2 beta metadata base advanced"]
		: [
				cargo,
				number + 1,
				`v${cargo}-beta.${number + 1}`,
				`R2 beta metadata ${version}`,
			];
}

const cargo = await cargoVersion();
const override = (Deno.env.get("BETA_VERSION_OVERRIDE") ?? "").trim();
let base: string;
let number: number;
let version: string;
let source: string;
if (override) {
	[base, number, version] = parseBeta(override, "BETA_VERSION_OVERRIDE");
	if (base !== cargo) {
		fail(`override base ${base} does not match Cargo version ${cargo}`);
	}
	source = "workflow override";
} else {
	[base, number, version, source] = await next(cargo);
}
console.log(`[release-beta] ${version} from ${source}`);
await output("base_version", base);
await output("beta_number", String(number));
await output("release_version", version);
await output("state_source", source);
