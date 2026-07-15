import { booleanOption, helpRequested, parseArgs } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const session = "openweb";
const home = ".local/playwright";
const table = "apps/react/src/lib/routes.ts";

function usage(): void {
  io.print("Usage: runseal :playwright <verb> [args]");
  io.print("Usage: runseal :playwright raw <playwright-cli args>");
  io.print("");
  io.print("Perceive the react app through a project-specialized playwright-cli session.");
  io.print("");
  io.print(`  shot <route...>   screenshot routes into ${home}/shots/<route>.png`);
  io.print("  shot --all        screenshot every route from the routes table");
  io.print(`  text <route...>   snapshot routes into ${home}/snaps/<route>.yml`);
  io.print("  console [level]   print console messages of the live page");
  io.print("  status            probe the app and list browser sessions");
  io.print("  close             close the browser session");
  io.print("  raw <args>        run raw playwright-cli inside the project session");
  io.print("");
  io.print("The app process belongs to sidecar; this wrapper never starts or stops it.");
}

type Slot = { running?: boolean; healthUrl?: string | null };

async function base(): Promise<string> {
  const text = await cmd.text("sidecar", ["status", "--format", "json"]);
  const status = JSON.parse(text) as { targets?: Slot[] };
  const found = (status.targets ?? []).find((slot) => typeof slot.healthUrl === "string");
  if (found === undefined || found.running !== true || typeof found.healthUrl !== "string") {
    return io.fail("playwright: the app is not running; start it through sidecar first");
  }
  return found.healthUrl.replace(/\/+$/, "");
}

async function routes(): Promise<string[]> {
  const text = await Deno.readTextFile(table);
  const paths = [...text.matchAll(/path:\s*"([^"]+)"/g)].map((found) => found[1]);
  if (paths.length === 0) {
    io.fail(`playwright: no routes found in ${table}`);
  }
  return paths;
}

function label(route: string): string {
  return route === "/" ? "home" : route.replace(/^\//, "").replaceAll("/", "-");
}

function resolve(wanted: string, paths: string[]): string {
  const found = paths.find((route) => route === wanted || label(route) === wanted);
  if (found === undefined) {
    return io.fail(
      `playwright: unknown route ${wanted}; table has ${paths.map(label).join(", ")}`,
    );
  }
  return found;
}

async function serving(url: string): Promise<boolean> {
  try {
    const response = await fetch(url, { headers: { "cache-control": "no-cache" } });
    await response.body?.cancel();
    return response.status === 200;
  } catch {
    return false;
  }
}

async function ensure(url: string): Promise<void> {
  if (!(await serving(url))) {
    io.fail(`playwright: nothing serving at ${url}; the app belongs to sidecar - start it there`);
  }
  await Deno.mkdir(`${home}/shots`, { recursive: true });
  await Deno.mkdir(`${home}/snaps`, { recursive: true });
  const config = { outputDir: `${home}/output` };
  await Deno.writeTextFile(`${home}/config.json`, `${JSON.stringify(config, null, "\t")}\n`);
}

function invocation(extra: string[]): string[] {
  return ["exec", "playwright-cli", `-s=${session}`, ...extra];
}

async function loud(extra: string[]): Promise<void> {
  await cmd.run("pnpm", invocation(extra));
}

async function quiet(extra: string[]): Promise<number> {
  return await cmd.status("pnpm", invocation(extra), { stdout: "null" });
}

async function visit(url: string): Promise<void> {
  if ((await quiet(["goto", url])) === 0) {
    return;
  }
  io.print(`  opening browser session ${session}`);
  const flags = ["--browser=chromium", `--config=${home}/config.json`];
  const code = await quiet(["open", ...flags, url]);
  if (code !== 0) {
    io.fail(`playwright: failed to open browser session (exit ${code})`);
  }
}

async function capture(names: string[], all: boolean, verb: "shot" | "text"): Promise<void> {
  const url = await base();
  await ensure(url);
  const paths = await routes();
  const wanted = all ? paths : names.map((name) => resolve(name, paths));
  if (wanted.length === 0) {
    io.fail(`playwright: ${verb} needs a route name or --all`);
  }
  for (const route of wanted) {
    await visit(`${url}${route}`);
    const file = verb === "shot"
      ? `${home}/shots/${label(route)}.png`
      : `${home}/snaps/${label(route)}.yml`;
    const order = verb === "shot" ? "screenshot" : "snapshot";
    const code = await quiet([order, `--filename=${file}`]);
    if (code !== 0) {
      io.fail(`playwright: ${order} failed for ${route} (exit ${code})`);
    }
    io.print(`${verb}: ${file}`);
  }
}

async function status(): Promise<void> {
  const url = await base();
  io.print(`app: ${(await serving(url)) ? "serving" : "down"} (${url})`);
  await cmd.run("pnpm", ["exec", "playwright-cli", "list"]);
}

if (Deno.args[0] === "raw") {
  if (Deno.args.length === 1) {
    usage();
    Deno.exit(1);
  }
  Deno.exit(await cmd.status("pnpm", invocation(Deno.args.slice(1))));
}

const args = parseArgs(Deno.args, { boolean: ["help", "h", "all"] });
if (helpRequested(args) || args._.length === 0) {
  usage();
  Deno.exit(0);
}

const verb = String(args._[0]);
const rest = args._.slice(1).map(String);

if (verb === "shot" || verb === "text") {
  await capture(rest, booleanOption(args, "all"), verb);
} else if (verb === "console") {
  await loud(["console", ...rest]);
} else if (verb === "status") {
  await status();
} else if (verb === "close") {
  await loud(["close"]);
} else {
  io.fail(`playwright: unknown verb: ${verb}`);
}
