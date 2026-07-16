import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

function usage(): void {
  io.print("Usage: runseal :bake");
  io.print("");
  io.print("Bake the living vocabulary and the stable release versions into apps/react/src/data.");
}

const args = parseArgs(Deno.args, { boolean: ["help", "h"] });
requireNoPositionals(args, "bake", { allowHelp: true });
if (helpRequested(args)) {
  usage();
  Deno.exit(0);
}

type Atom = { word: string; count: number };
type Root = { root: string; atoms: Atom[] };
type Entry = { repo: string; roots: Root[] };

const repos = ["negentropy", "runseal", "sidecar", "open-web"];
const target = "apps/react/src/data/vocabulary.json";
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
  const text = await cmd.text("negentropy", ["--vocabulary", "."], { cwd: dir });
  return { repo, roots: parse(text) };
}

const entries: Entry[] = [];
for (const repo of repos) {
  entries.push(await gather(repo));
}

await Deno.mkdir("apps/react/src/data", { recursive: true });
await Deno.writeTextFile(target, `${JSON.stringify(entries, null, "\t")}\n`);
await cmd.run("pnpm", ["biome", "format", "--write", target]);

const atoms = entries
  .flatMap((entry) => entry.roots)
  .reduce((sum, root) => sum + root.atoms.length, 0);
io.print(`bake: ${atoms} atoms across ${entries.length} repos -> ${target}`);

const releases: Record<string, Gate> = {};
for (const [name, base] of Object.entries(gates)) {
  releases[name] = await gate(base);
}
await Deno.writeTextFile(shelf, `${JSON.stringify(releases, null, "\t")}\n`);
await cmd.run("pnpm", ["biome", "format", "--write", shelf]);
const summary = Object.values(releases)
  .map((entry) => entry.version)
  .join(" ");
io.print(`bake: ${summary} -> ${shelf}`);
