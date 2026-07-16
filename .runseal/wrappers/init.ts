import { cli, flags } from "@perish/harness/cli";
import { init } from "@perish/harness/init";
import { io } from "@perish/harness/io";

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
flags(args).positionals("init", { allowHelp: true });
if (flags(args).help()) {
  io.print("Usage: runseal :init");
  io.print("");
  io.print("Validate the repository and install versioned git hooks.");
  Deno.exit(0);
}

await init({
  tools: ["git", "tea", "deno", "node", "pnpm", "negentropy", "runseal", "sh", "bash"],
  paths: [
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
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".forgejo/workflows/guard.yml",
  ],
});
