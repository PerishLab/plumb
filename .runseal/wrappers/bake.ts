import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

function usage(): void {
  io.print("Usage: runseal :bake");
  io.print("");
  io.print("Bake the living vocabulary of the four repos into apps/react/src/data.");
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
