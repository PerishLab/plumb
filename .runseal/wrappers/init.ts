import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { fs } from "@/lib/std/fs.ts";
import { io } from "@/lib/std/io.ts";
import { path } from "@/lib/std/path.ts";

const HOOKS_PATH = ".runseal/hooks";

function usage(): void {
  io.print("Usage: runseal :init");
  io.print("");
  io.print("Validate the repository and install versioned git hooks.");
}

async function requireTool(name: string): Promise<void> {
  if (!(await cmd.exists(name))) {
    io.fail(`init: missing required tool: ${name}`);
  }
}

async function requirePath(root: string, relPath: string): Promise<void> {
  if (!(await fs.file.exists(path.join(root, relPath)))) {
    io.fail(`init: missing required path: ${relPath}`);
  }
}

const args = parseArgs(Deno.args, { boolean: ["help", "h"] });
requireNoPositionals(args, "init", { allowHelp: true });
if (helpRequested(args)) {
  usage();
  Deno.exit(0);
}

io.print("==> resolving repository");
const root = await cmd.text("git", ["rev-parse", "--show-toplevel"]);
io.print(`repository: ${root}`);

io.print("==> checking required tools");
for (const tool of ["git", "tea", "deno", "node", "pnpm", "negentropy", "runseal", "sh", "bash"]) {
  await requireTool(tool);
}
io.print("ok: git, tea, deno, node, pnpm, negentropy, runseal, sh, bash");

io.print("==> checking repository entrypoints");
for (
  const entry of [
    "package.json",
    "pnpm-workspace.yaml",
    "biome.json",
    "negentropy.toml",
    "vocabulary.toml",
    "runseal.toml",
    "sidecar.toml",
    "apps/react/package.json",
    "packages/components/package.json",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/negentropy.version",
    ".runseal/hooks/pre-commit",
    ".runseal/hooks/commit-msg",
    ".runseal/lib/cli.ts",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".forgejo/workflows/guard.yml",
  ]
) {
  await requirePath(root, entry);
}
io.print("ok: repository entrypoints");

io.print("==> installing git hooks");
await cmd.run("git", ["config", "core.hooksPath", HOOKS_PATH], { cwd: root });
const current = await cmd.text("git", ["config", "--get", "core.hooksPath"], { cwd: root });
io.print(`core.hooksPath = ${current}`);

await cmd.run("deno", ["--version"], { stdout: "null" });
io.print("development environment ready");
