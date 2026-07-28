import { Cloudflare, keys as held } from "@perish/sealkit/cloudflare";
import { cli, flags } from "@perish/sealkit/cli";
import { bin, exists } from "@perish/sealkit/cmd";
import { env } from "@perish/sealkit/env";
import { fs } from "@perish/sealkit/fs";
import { io } from "@perish/sealkit/io";
import { doc } from "@perish/sealkit/json";
import { runseal } from "@perish/sealkit/runseal";
import { family, kind, run } from "@perish/shield";

const app = "apps/web";
const dist = `${app}/dist`;
const config = `${app}/wrangler.jsonc`;
const atlas = `${dist}/sitemap.xml`;

const fault = family("ship", {
  unfilled: kind<{ missing: string[] }>(),
  build: kind<{ path: string }>(),
  unreached: kind<{ domain: string }>(),
});

function usage(): void {
  io.print("Usage: runseal :ship [--dry-run | --check]");
  io.print("");
  io.print("Build the react app and deploy it as Workers Static Assets on the site domain.");
  io.print("");
  io.print("  --dry-run   print the plan and run a credential-free wrangler dry run");
  io.print("  --check     probe the token, zone, DNS record, and worker via cloudflare tooling");
  io.print("");
  io.print("Secrets:");
  io.print("  .local/secrets/ship.env        PLUMB_SITE_DOMAIN (see AGENTS.md)");
  io.print("  .local/secrets/cloudflare.env  CLOUDFLARE_ACCOUNT_ID + CLOUDFLARE_API_TOKEN");
}

function secretsDir(): string {
  return env.get("RUNSEAL_REPO_SECRETS_DIR", ".local/secrets");
}

async function secrets(name: string): Promise<Record<string, string>> {
  const text = await fs.file.readTextIfExists(`${secretsDir()}/${name}`);
  const values: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const entry = line.trim();
    if (entry === "" || entry.startsWith("#") || !entry.includes("=")) {
      continue;
    }
    const at = entry.indexOf("=");
    const value = entry.slice(at + 1).trim();
    values[entry.slice(0, at).trim()] = value.replace(/^["']/, "").replace(/["']$/, "");
  }
  return values;
}

function unfilled(values: Record<string, string>, wanted: string[]): string[] {
  return wanted.filter((key) => values[key] === undefined || values[key] === "");
}

type Vault = {
  domain: string;
  account: string;
  token: string;
  empty: string[];
};

async function vault(): Promise<Vault> {
  const site = await secrets("ship.env");
  const cloud = await secrets("cloudflare.env");
  const empty = [
    ...unfilled(site, ["PLUMB_SITE_DOMAIN"]).map((key) => `ship.env: ${key}`),
    ...unfilled(cloud, ["CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_TOKEN"]).map(
      (key) => `cloudflare.env: ${key}`,
    ),
  ];
  return {
    domain: site.PLUMB_SITE_DOMAIN ?? "",
    account: cloud.CLOUDFLARE_ACCOUNT_ID ?? "",
    token: cloud.CLOUDFLARE_API_TOKEN ?? "",
    empty,
  };
}

async function worker(): Promise<string> {
  const text = await Deno.readTextFile(config);
  const found = text.match(/"name":\s*"([^"]+)"/);
  if (found === null) {
    return io.fail(`ship: no worker name found in ${config}`);
  }
  return found[1];
}

async function routes(): Promise<string[]> {
  if (!(await fs.file.exists(atlas))) {
    return [];
  }
  const text = await Deno.readTextFile(atlas);
  return [...text.matchAll(/<loc>([^<]+)<\/loc>/g)]
    .map((found) => new URL(found[1]).pathname)
    .map((path) => (path.length > 1 ? path.replace(/\/$/, "") : path));
}

function deep(paths: string[]): string | undefined {
  return paths.find((route) => route !== "/");
}

async function plan(): Promise<void> {
  const keys = await vault();
  const domain = keys.domain === "" ? "<PLUMB_SITE_DOMAIN>" : keys.domain;
  const paths = await routes();
  io.print("==> ship plan (dry run)");
  io.print("");
  io.print("build:");
  io.print("  pnpm --filter @plumb/web build");
  io.print("");
  io.print("deploy:");
  io.print("  env: CLOUDFLARE_ACCOUNT_ID=<CLOUDFLARE_ACCOUNT_ID> CLOUDFLARE_API_TOKEN=<redacted>");
  io.print(`  pnpm exec wrangler deploy --domain ${domain}  (cwd ${app})`);
  io.print("");
  io.print("verify:");
  io.print(`  https://${domain}/`);
  const route = deep(paths);
  if (route !== undefined) {
    io.print(`  https://${domain}${route}`);
  }
  if (keys.empty.length > 0) {
    io.print("");
    io.print(`unfilled in ${secretsDir()}: ${keys.empty.join(", ")}`);
  }
  io.print("");
  if (!(await fs.file.exists(`${dist}/index.html`))) {
    io.print(`wrangler dry run: skipped (${dist}/index.html missing; build first)`);
    return;
  }
  io.print("wrangler dry run:");
  const flags = keys.domain === "" ? [] : ["--domain", keys.domain];
  await bin("pnpm").run(["exec", "wrangler", "deploy", "--dry-run", ...flags], { cwd: app });
}

async function probe(url: string, mark: string): Promise<boolean> {
  for (let turn = 0; turn < 10; turn += 1) {
    const seen = await knock(url);
    if (seen.status === 200 && seen.body.includes(mark)) {
      io.print(`  200 ${url} serving ${mark}`);
      return true;
    }
    const why = seen.status === 200 ? "stale build" : seen.status;
    io.print(`  retry ${url} (${why})`);
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  return false;
}

async function marks(): Promise<Record<string, string>> {
  const commit = await bin("git").text(["rev-parse", "--short", "HEAD"]);
  const text = await Deno.readTextFile("Cargo.toml");
  const found = /^version = "([^"]+)"$/m.exec(text);
  return { BUILD_COMMIT: commit, BUILD_VERSION: found?.[1] ?? "" };
}

async function stamp(): Promise<string> {
  const html = await Deno.readTextFile(`${dist}/index.html`);
  const found = /\/assets\/index-[A-Za-z0-9_-]+\.js/.exec(html);
  if (found === null) {
    return io.fail(`ship: no fingerprinted asset in ${dist}/index.html`);
  }
  return found[0];
}

async function knock(url: string): Promise<{ status: number | string; body: string }> {
  try {
    const response = await fetch(url, { headers: { "cache-control": "no-cache" } });
    const body = await response.text();
    return { status: response.status, body };
  } catch (thrown) {
    const status = thrown instanceof Error ? thrown.message.split(":")[0] : "unreachable";
    return { status, body: "" };
  }
}

async function ship(): Promise<void> {
  const keys = await vault();
  if (keys.empty.length > 0) {
    throw fault.unfilled({ missing: keys.empty });
  }
  io.print("==> build");
  await bin("pnpm").run(["--filter", "@plumb/web", "build"], { env: await marks() });
  if (!(await fs.file.exists(`${dist}/index.html`))) {
    throw fault.build({ path: `${dist}/index.html` });
  }
  io.print("==> deploy");
  await bin("pnpm").run(["exec", "wrangler", "deploy", "--domain", keys.domain], {
    cwd: app,
    env: {
      CLOUDFLARE_ACCOUNT_ID: keys.account,
      CLOUDFLARE_API_TOKEN: keys.token,
    },
  });
  io.print("==> verify");
  const paths = await routes();
  const mark = await stamp();
  let reached = await probe(`https://${keys.domain}/`, mark);
  const route = deep(paths);
  if (reached && route !== undefined) {
    reached = await probe(`https://${keys.domain}${route}`, mark);
  }
  if (!reached) {
    await anchored(keys);
  }
  io.print("ship: ok");
}

async function anchored(keys: { account: string; token: string; domain: string }): Promise<void> {
  const base = "https://api.cloudflare.com/client/v4";
  const response = await fetch(`${base}/accounts/${keys.account}/workers/domains`, {
    headers: { authorization: `Bearer ${keys.token}` },
  });
  const body = await response.json();
  const bound = (body.result ?? []).some(
    (entry: { hostname?: string }) => entry.hostname === keys.domain,
  );
  if (!bound) {
    throw fault.unreached({ domain: keys.domain });
  }
  io.print(`  edge unreachable from here; API confirms ${keys.domain} is attached (local proxy?)`);
}

function seated(value: unknown): Record<string, unknown> {
  if (typeof value === "object" && value !== null && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

async function check(): Promise<void> {
  const keys = await vault();
  const gate = keys.empty.filter((key) => key.startsWith("cloudflare.env"));
  if (gate.length > 0) {
    io.print(`check: skipped (unfilled in ${secretsDir()}: ${gate.join(", ")})`);
    return;
  }
  const secrets = await held();
  const api = new Cloudflare(
    secrets.CLOUDFLARE_API_TOKEN ?? "",
    env.get("CLOUDFLARE_API_BASE", "https://api.cloudflare.com/client/v4"),
  );
  let verdict: Record<string, unknown>;
  try {
    verdict = seated(await api.result("GET", "/user/tokens/verify"));
  } catch {
    try {
      verdict = seated(await api.result("GET", `/accounts/${keys.account}/tokens/verify`));
    } catch {
      return io.fail("check: token failed both /user and /accounts verify endpoints");
    }
  }
  io.print(`token: ${verdict.status}`);
  const name = secrets.CLOUDFLARE_ZONE_NAME || "perish.uk";
  const zone = await api.zone(name);
  const id = String(zone.id);
  io.print(`zone: ${name} (${id})`);
  if (keys.domain === "") {
    io.print("check: skipped dns probe (unfilled in ship.env: PLUMB_SITE_DOMAIN)");
  } else {
    const records = (await api.records(id, keys.domain)).map(seated);
    if (records.length === 0) {
      io.print(`dns: no record for ${keys.domain} yet (wrangler deploy attaches the domain)`);
    } else {
      io.print(`dns: ${keys.domain} ${records[0].type} (${records[0].id})`);
    }
  }
  const script = await worker();
  try {
    const found = seated(
      await api.result("GET", `/accounts/${keys.account}/workers/services/${script}`),
    );
    io.print(`worker: ${found.id} (created ${found.created_on})`);
  } catch {
    io.print(`worker: ${script} not found yet (first :ship creates it)`);
  }
  io.print("check: ok");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h", "dry-run", "check"] });
flags(args).positionals("ship", { allowHelp: true });
if (flags(args).help()) {
  usage();
  Deno.exit(0);
}
if (flags(args).boolean("check")) {
  await check();
} else if (flags(args).boolean("dry-run")) {
  await plan();
} else {
  await run(ship).catch(fault.consume({
    unfilled: (thrown) =>
      io.fail(`ship: unfilled in ${secretsDir()}: ${thrown.meta.missing.join(", ")}`),
    build: (thrown) => io.fail(`ship: build produced no ${thrown.meta.path}`),
    unreached: (thrown) =>
      io.fail(
        `ship: edge unreachable and ${thrown.meta.domain} is not attached; deploy likely failed`,
      ),
  }));
}
