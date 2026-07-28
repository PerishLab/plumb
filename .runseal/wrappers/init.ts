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
    "manage.sh",
    "manage.ps1",
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
    ".forgejo/release.env.example",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-beta.yml",
    ".forgejo/workflows/release-stable.yml",
    ".forgejo/workflows/release-verify.yml",
    ".forgejo/scripts/release/assets/checksums.sh",
    ".forgejo/scripts/release/assets/package.ps1",
    ".forgejo/scripts/release/assets/package.sh",
    ".forgejo/scripts/release/assets/verify.sh",
    ".forgejo/scripts/release/cargo/publish.ts",
    ".forgejo/scripts/release/metadata/beta.ts",
    ".forgejo/scripts/release/metadata/stable.ts",
    ".forgejo/scripts/release/r2/check.sh",
    ".forgejo/scripts/release/r2/publish.sh",
    ".forgejo/scripts/release/r2/summary.sh",
    ".forgejo/scripts/release/r2/verify.sh",
    ".forgejo/scripts/release/smoke/local.sh",
    ".forgejo/scripts/release/smoke/smoke.ps1",
    ".forgejo/scripts/release/smoke/smoke.sh",
  ],
});
