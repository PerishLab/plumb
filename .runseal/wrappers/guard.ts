import { cache } from "@perish/harness/cache";
import { cli, flags } from "@perish/harness/cli";
import { bin, exists } from "@perish/harness/cmd";
import { io } from "@perish/harness/io";

function usage(): void {
  io.print("Usage: runseal :guard [--fresh]");
  io.print("");
  io.print("Run repository guard checks.");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h", "fresh"] });
flags(args).positionals("guard", { allowHelp: true });
if (flags(args).help()) {
  usage();
  Deno.exit(0);
}

async function pin(): Promise<void> {
  const want = (await Deno.readTextFile(".runseal/negentropy.version")).trim();
  const have = (await bin("negentropy").text(["--version"])).replace("negentropy", "").trim();
  if (have !== want) {
    io.fail(`guard: negentropy ${have} does not match pin ${want}`);
  }
}

const mark = await cache.key();
if (args.fresh !== true && (await cache.hit(mark))) {
  io.print(`guard: clean (cached ${mark.slice(0, 12)})`);
  Deno.exit(0);
}

io.print("==> negentropy version pin");
await pin();

io.print("==> biome");
await bin("pnpm").run(["biome", "ci", "."]);

io.print("==> tsc");
await bin("pnpm").run(["-r", "exec", "tsc", "--noEmit"]);

io.print("==> vitest");
await bin("pnpm").run(["-r", "test"]);

io.print("==> deno fmt");
await bin("deno").run(["fmt", "--check", ".runseal"]);

io.print("==> deno check");
await bin("deno").run([
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
await bin("negentropy").run(["--strict", "."]);

await cache.keep(mark);
