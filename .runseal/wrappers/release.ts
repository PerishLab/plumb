import { cli, flags } from "@perish/sealkit/cli";
import { bin } from "@perish/sealkit/cmd";
import { io } from "@perish/sealkit/io";

function usage(): void {
  io.print("Usage: runseal :release <command> [args]");
  io.print("");
  io.print("Run the product release coordinator from the local build.");
  io.print("Registry publish lives under `registry`; `:release help` lists the rest.");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (args._.length === 0 && flags(args).help()) {
  usage();
  Deno.exit(0);
}

await bin("cargo").run([
  "run",
  "--quiet",
  "-p",
  "plumb-cli",
  "--",
  "release",
  ...Deno.args,
]);
