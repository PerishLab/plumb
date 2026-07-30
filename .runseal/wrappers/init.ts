import { cli, flags } from "@perish/sealkit/cli";
import { init } from "@perish/sealkit/init";
import { io } from "@perish/sealkit/io";

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
flags(args).positionals("init", { allowHelp: true });
if (flags(args).help()) {
  io.print("Usage: runseal :init");
  io.print("");
  io.print("Validate the repository and install versioned git hooks.");
  Deno.exit(0);
}

await init({
  tools: ["git", "tea", "deno", "node", "pnpm", "ectropy", "plumb", "runseal", "sh", "bash"],
  paths: [
    "package.json",
    "pnpm-workspace.yaml",
    "biome.json",
    "ectropy.toml",
    "runseal.toml",
    "sidecar.toml",
    "apps/web/package.json",
    "apps/web/vite.config.ts",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/hooks/pre-commit",
    ".runseal/hooks/commit-msg",
    ".runseal/lib/cold-start/project.ts",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/cold-start.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".runseal/wrappers/release.ts",
    ".runseal/wrappers/retire.ts",
    ".runseal/wrappers/ship.ts",
    "plumb.toml",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-exact.yml",
    ".forgejo/workflows/release-stable.yml",
  ],
});
