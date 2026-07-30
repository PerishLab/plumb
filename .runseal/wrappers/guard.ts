import { guard } from "@perish/sealkit/guard";

await guard(
  [
    { label: "cargo fmt", runs: [["cargo", ["fmt", "--all", "--check"]]] },
    {
      label: "cargo clippy",
      runs: [["cargo", ["clippy", "--all-targets", "--", "-D", "warnings"]]],
    },
    { label: "cargo test", runs: [["cargo", ["test", "--locked"]]] },
    { label: "biome", runs: [["pnpm", ["biome", "ci", "."]]] },
    { label: "tsc", runs: [["pnpm", ["-r", "exec", "tsc", "--noEmit"]]] },
    { label: "vitest", runs: [["pnpm", ["-r", "test"]]] },
    {
      label: "web build",
      runs: [["pnpm", ["--filter", "@plumb/web", "build"]]],
    },
    {
      label: "plumb doctor",
      runs: [["cargo", ["run", "--quiet", "-p", "plumb-cli", "--", "doctor", "."]]],
    },
    {
      label: "deno fmt",
      runs: [["deno", ["fmt", "--check", ".runseal"]]],
    },
    {
      label: "deno check",
      runs: [["deno", [
        "check",
        "--config",
        ".runseal/deno.json",
        "--lock",
        ".runseal/deno.lock",
        "--frozen=true",
        ".runseal/wrappers/guard.ts",
        ".runseal/wrappers/cold-start.ts",
        ".runseal/wrappers/init.ts",
        ".runseal/wrappers/land.ts",
        ".runseal/wrappers/release.ts",
        ".runseal/wrappers/retire.ts",
        ".runseal/wrappers/ship.ts",
      ]]],
    },
  ],
  Deno.args,
  { checker: ["ectropy", ["."]] },
);
