import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

function usage(): void {
  io.print("Usage: runseal :guard");
  io.print("");
  io.print("Run repository guard checks.");
}

const args = parseArgs(Deno.args, { boolean: ["help", "h"] });
requireNoPositionals(args, "guard", { allowHelp: true });
if (helpRequested(args)) {
  usage();
  Deno.exit(0);
}

async function pin(): Promise<void> {
  const want = (await Deno.readTextFile(".runseal/negentropy.version")).trim();
  const have = (await cmd.text("negentropy", ["--version"])).replace("negentropy", "").trim();
  if (have !== want) {
    io.fail(`guard: negentropy ${have} does not match pin ${want}`);
  }
}

io.print("==> negentropy version pin");
await pin();

io.print("==> biome");
await cmd.run("pnpm", ["biome", "ci", "."]);

io.print("==> tsc");
await cmd.run("pnpm", ["-r", "exec", "tsc", "--noEmit"]);

io.print("==> vitest");
await cmd.run("pnpm", ["-r", "test"]);

io.print("==> deno fmt");
await cmd.run("deno", ["fmt", "--check", ".runseal"]);

io.print("==> deno check");
await cmd.run("deno", [
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
]);

io.print("==> negentropy");
await cmd.run("negentropy", ["--strict", "."]);
