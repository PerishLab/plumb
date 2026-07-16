import { cli, flags } from "@perish/harness/cli";
import { bin, exists } from "@perish/harness/cmd";
import { io } from "@perish/harness/io";

function usage(): void {
  io.print("Usage: runseal :bake");
  io.print("");
  io.print("Bake the living vocabulary, the stable release versions, and the");
  io.print("negentropy transcripts into apps/react/src/data.");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
flags(args).positionals("bake", { allowHelp: true });
if (flags(args).help()) {
  usage();
  Deno.exit(0);
}

type Atom = { word: string; count: number };
type Root = { root: string; atoms: Atom[] };
type Entry = { repo: string; roots: Root[] };

const repos = ["negentropy", "runseal", "sidecar", "open-web"];
const target = "apps/react/src/data/vocabulary.json";
const scroll = "apps/react/src/data/transcripts.json";
const shelf = "apps/react/src/data/releases.json";
const gates: Record<string, string> = {
  negentropy: "https://releases.negentropy.perish.uk",
  runseal: "https://releases.runseal.perish.uk",
  sidecar: "https://releases.sidecar.perish.uk",
};

type Gate = { version: string; platforms: string; windows: boolean };

function phrase(keys: string[]): string {
  const parts: string[] = [];
  if (keys.includes("linuxX64")) {
    parts.push("linux x86_64");
  }
  const arm = keys.includes("macArm64") || keys.includes("darwinArm64");
  const intel = keys.includes("macX64") || keys.includes("darwinX64");
  if (arm && intel) {
    parts.push("macos (intel and apple silicon)");
  } else if (arm) {
    parts.push("macos (apple silicon)");
  } else if (intel) {
    parts.push("macos (intel)");
  }
  return parts.join(" and ");
}

async function gate(base: string): Promise<Gate> {
  const response = await fetch(`${base}/stable/latest/metadata.json`);
  if (!response.ok) {
    io.fail(`bake: ${base} answered ${response.status}`);
  }
  const body = await response.json();
  const keys = Object.keys(body.artifacts ?? {});
  return {
    version: typeof body.releaseVersion === "string" ? body.releaseVersion : "",
    platforms: phrase(keys),
    windows: keys.includes("winX64") || keys.includes("windowsX64"),
  };
}

const laws = `[scan]
include = ["src/**/*.ts"]

[module]
roots = ["src"]

[limit]
block = 4
path = 4

[comment]
allow = false

[word]
single = true`;

const dirty = `export function fetchAllData(rows: number[][][]): number {
	let sum = 0;
	for (const plane of rows) {
		for (const row of plane) {
			for (const cell of row) {
				if (cell > 0) {
					sum += cell;
				}
			}
		}
	}
	return sum;
}`;

const tidy = `export function tally(rows: number[][][]): number {
	let sum = 0;
	for (const plane of rows) {
		sum += gain(plane);
	}
	return sum;
}

function gain(plane: number[][]): number {
	let sum = 0;
	for (const row of plane) {
		for (const cell of row) {
			if (cell > 0) {
				sum += cell;
			}
		}
	}
	return sum;
}`;

type Scroll = { laws: string; dirty: string; finding: string; mended: string; clean: string };

async function witness(): Promise<Scroll> {
  await Deno.mkdir(".local/tmp", { recursive: true });
  const dir = await Deno.makeTempDir({ dir: ".local/tmp" });
  try {
    await Deno.mkdir(`${dir}/src`, { recursive: true });
    await Deno.writeTextFile(`${dir}/negentropy.toml`, `${laws}\n`);
    await Deno.writeTextFile(`${dir}/src/helper.ts`, `${dirty}\n`);
    const finding = await bin("sh").text(["-c", "negentropy --debt . || true"], { cwd: dir });
    if (!finding.includes("fault") || !finding.includes("debt")) {
      io.fail(`bake: dirty fixture did not produce a finding transcript:\n${finding}`);
    }
    await Deno.writeTextFile(`${dir}/src/helper.ts`, `${tidy}\n`);
    const verdict = await bin("negentropy").text(["--strict", "."], { cwd: dir });
    if (verdict !== "clean") {
      io.fail(`bake: mended fixture did not come back clean:\n${verdict}`);
    }
    return {
      laws,
      dirty,
      finding: `$ negentropy --debt .\n${finding}`,
      mended: tidy,
      clean: `$ negentropy --strict .\n${verdict}`,
    };
  } finally {
    await Deno.remove(dir, { recursive: true });
  }
}

function parse(text: string): Root[] {
  const roots: Root[] = [];
  for (const line of text.split("\n")) {
    if (line.trim() === "") {
      continue;
    }
    if (line.startsWith("  ")) {
      const [word, count] = line.trim().split(" ");
      roots.at(-1)?.atoms.push({ word, count: Number(count) });
    } else {
      roots.push({ root: line.trim(), atoms: [] });
    }
  }
  return roots.filter((root) => root.atoms.length > 0);
}

async function gather(repo: string): Promise<Entry> {
  const dir = repo === "open-web" ? "." : `../${repo}`;
  try {
    await Deno.stat(`${dir}/negentropy.toml`);
  } catch {
    io.fail(`bake: ${dir} is not a negentropy-guarded checkout`);
  }
  io.print(`==> ${repo}`);
  const text = await bin("negentropy").text(["--vocabulary", "."], { cwd: dir });
  return { repo, roots: parse(text) };
}

const entries: Entry[] = [];
for (const repo of repos) {
  entries.push(await gather(repo));
}

await Deno.mkdir("apps/react/src/data", { recursive: true });
await Deno.writeTextFile(target, `${JSON.stringify(entries, null, "\t")}\n`);
await bin("pnpm").run(["biome", "format", "--write", target]);

const atoms = entries
  .flatMap((entry) => entry.roots)
  .reduce((sum, root) => sum + root.atoms.length, 0);
io.print(`bake: ${atoms} atoms across ${entries.length} repos -> ${target}`);

const witnessed = { negentropy: await witness() };
await Deno.writeTextFile(scroll, `${JSON.stringify(witnessed, null, "\t")}\n`);
await bin("pnpm").run(["biome", "format", "--write", scroll]);
io.print(`bake: negentropy transcripts witnessed -> ${scroll}`);

const releases: Record<string, Gate> = {};
for (const [name, base] of Object.entries(gates)) {
  releases[name] = await gate(base);
}
await Deno.writeTextFile(shelf, `${JSON.stringify(releases, null, "\t")}\n`);
await bin("pnpm").run(["biome", "format", "--write", shelf]);
const summary = Object.values(releases)
  .map((entry) => entry.version)
  .join(" ");
io.print(`bake: ${summary} -> ${shelf}`);
