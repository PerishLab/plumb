import { booleanOption, helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { env } from "@/lib/std/env.ts";
import { fs } from "@/lib/std/fs.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { runseal } from "@/lib/std/runseal.ts";

const app = "apps/react";
const dist = `${app}/dist`;
const config = `${app}/wrangler.jsonc`;
const table = `${app}/src/lib/routes.ts`;

function usage(): void {
  io.print("Usage: runseal :ship [--dry-run | --check]");
  io.print("");
  io.print("Build the react app and deploy it as Workers Static Assets on the site domain.");
  io.print("");
  io.print("  --dry-run   print the plan and run a credential-free wrangler dry run");
  io.print("  --check     probe the token, zone, DNS record, and worker via cloudflare tooling");
  io.print("");
  io.print("Secrets:");
  io.print("  .local/secrets/ship.env        OPENWEB_SITE_DOMAIN (see AGENTS.md)");
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
    ...unfilled(site, ["OPENWEB_SITE_DOMAIN"]).map((key) => `ship.env: ${key}`),
    ...unfilled(cloud, ["CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_TOKEN"]).map(
      (key) => `cloudflare.env: ${key}`,
    ),
  ];
  return {
    domain: site.OPENWEB_SITE_DOMAIN ?? "",
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
  const text = await Deno.readTextFile(table);
  const paths = [...text.matchAll(/path:\s*"([^"]+)"/g)].map((found) => found[1]);
  if (paths.length === 0) {
    io.fail(`ship: no routes found in ${table}`);
  }
  return paths;
}

function deep(paths: string[]): string | undefined {
  return paths.find((route) => route !== "/");
}

async function plan(): Promise<void> {
  const keys = await vault();
  const domain = keys.domain === "" ? "<OPENWEB_SITE_DOMAIN>" : keys.domain;
  const paths = await routes();
  io.print("==> ship plan (dry run)");
  io.print("");
  io.print("build:");
  io.print("  pnpm --filter @open-web/react build");
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
  await cmd.run("pnpm", ["exec", "wrangler", "deploy", "--dry-run", ...flags], { cwd: app });
}

async function probe(url: string): Promise<boolean> {
  for (let turn = 0; turn < 3; turn += 1) {
    const status = await knock(url);
    if (status === 200) {
      io.print(`  200 ${url}`);
      return true;
    }
    io.print(`  retry ${url} (${status})`);
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  return false;
}

async function knock(url: string): Promise<number | string> {
  try {
    const response = await fetch(url, { headers: { "cache-control": "no-cache" } });
    await response.body?.cancel();
    return response.status;
  } catch (thrown) {
    return thrown instanceof Error ? thrown.message.split(":")[0] : "unreachable";
  }
}

async function ship(): Promise<void> {
  const keys = await vault();
  if (keys.empty.length > 0) {
    io.fail(`ship: unfilled in ${secretsDir()}: ${keys.empty.join(", ")}`);
  }
  const paths = await routes();
  io.print("==> build");
  await cmd.run("pnpm", ["--filter", "@open-web/react", "build"]);
  if (!(await fs.file.exists(`${dist}/index.html`))) {
    io.fail(`ship: build produced no ${dist}/index.html`);
  }
  io.print("==> deploy");
  await cmd.run("pnpm", ["exec", "wrangler", "deploy", "--domain", keys.domain], {
    cwd: app,
    env: {
      CLOUDFLARE_ACCOUNT_ID: keys.account,
      CLOUDFLARE_API_TOKEN: keys.token,
    },
  });
  io.print("==> verify");
  let reached = await probe(`https://${keys.domain}/`);
  const route = deep(paths);
  if (reached && route !== undefined) {
    reached = await probe(`https://${keys.domain}${route}`);
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
    io.fail(`ship: edge unreachable and ${keys.domain} is not attached; deploy likely failed`);
  }
  io.print(`  edge unreachable from here; API confirms ${keys.domain} is attached (local proxy?)`);
}

async function attempt(args: string[]): Promise<{ code: number; out: string }> {
  const output = await new Deno.Command("runseal", {
    args,
    stdin: "null",
    stdout: "piped",
    stderr: "piped",
  }).output();
  return { code: output.code, out: new TextDecoder().decode(output.stdout).trimEnd() };
}

async function check(): Promise<void> {
  const keys = await vault();
  const gate = keys.empty.filter((key) => key.startsWith("cloudflare.env"));
  if (gate.length > 0) {
    io.print(`check: skipped (unfilled in ${secretsDir()}: ${gate.join(", ")})`);
    return;
  }
  let verdict = await attempt([
    "@tool",
    "cloudflare",
    "api",
    "request",
    "GET",
    "/user/tokens/verify",
  ]);
  if (verdict.code !== 0) {
    verdict = await attempt([
      "@tool",
      "cloudflare",
      "api",
      "request",
      "GET",
      `/accounts/${keys.account}/tokens/verify`,
    ]);
  }
  if (verdict.code !== 0) {
    io.fail("check: token failed both /user and /accounts verify endpoints");
  }
  io.print(`token: ${json.get(verdict.out, ".result.status")}`);
  const name = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", name]);
  const id = json.get(zone, ".id");
  io.print(`zone: ${name} (${id})`);
  if (keys.domain === "") {
    io.print("check: skipped dns probe (unfilled in ship.env: OPENWEB_SITE_DOMAIN)");
  } else {
    const records = await runseal.text([
      "@tool",
      "cloudflare",
      "zone",
      "dns-record",
      "list",
      "--zone-id",
      id,
      "--name",
      keys.domain,
    ]);
    if (json.len(records) === 0) {
      io.print(`dns: no record for ${keys.domain} yet (wrangler deploy attaches the domain)`);
    } else {
      const record = json.get(records, "[0]");
      io.print(`dns: ${keys.domain} ${json.get(record, ".type")} (${json.get(record, ".id")})`);
    }
  }
  const script = await worker();
  const service = await attempt([
    "@tool",
    "cloudflare",
    "api",
    "request",
    "GET",
    `/accounts/${keys.account}/workers/services/${script}`,
  ]);
  if (service.code === 0) {
    const found = json.get(service.out, ".result");
    io.print(`worker: ${json.get(found, ".id")} (created ${json.get(found, ".created_on")})`);
  } else {
    io.print(`worker: ${script} not found yet (first :ship creates it)`);
  }
  io.print("check: ok");
}

const args = parseArgs(Deno.args, { boolean: ["help", "h", "dry-run", "check"] });
requireNoPositionals(args, "ship", { allowHelp: true });
if (helpRequested(args)) {
  usage();
  Deno.exit(0);
}
if (booleanOption(args, "check")) {
  await check();
} else if (booleanOption(args, "dry-run")) {
  await plan();
} else {
  await ship();
}
