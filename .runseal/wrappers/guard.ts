import { guard } from "@perish/harness/guard";

await guard([
  { label: "biome", runs: [["pnpm", ["biome", "ci", "."]]] },
  { label: "tsc", runs: [["pnpm", ["-r", "exec", "tsc", "--noEmit"]]] },
  { label: "vitest", runs: [["pnpm", ["-r", "test"]]] },
  { label: "deno fmt", runs: [["deno", ["fmt", "--check", ".runseal"]]] },
  {
    label: "deno check",
    runs: [["deno", [
      "check",
      "--config",
      ".runseal/deno.json",
      "--lock",
      ".runseal/deno.lock",
      "--frozen=true",
      ".runseal/wrappers/bake.ts",
      ".runseal/wrappers/guard.ts",
      ".runseal/wrappers/init.ts",
      ".runseal/wrappers/land.ts",
      ".runseal/wrappers/playwright.ts",
      ".runseal/wrappers/ship.ts",
    ]]],
  },
], Deno.args);
