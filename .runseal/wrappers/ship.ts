import { booleanOption, helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { env } from "@/lib/std/env.ts";
import { fs } from "@/lib/std/fs.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { runseal } from "@/lib/std/runseal.ts";

const dist = "apps/react/dist";
const table = "apps/react/src/lib/routes.ts";
const keys = [
  "OPENWEB_SITE_S3_AK",
  "OPENWEB_SITE_S3_SK",
  "OPENWEB_SITE_S3_BUCKET",
  "OPENWEB_SITE_S3_URL",
  "OPENWEB_SITE_DOMAIN",
];

function usage(): void {
  io.print("Usage: runseal :ship [--dry-run | --check]");
  io.print("");
  io.print("Build the react app, snapshot SPA routes, and sync dist/ to the R2 site bucket.");
  io.print("");
  io.print("  --dry-run   print the full plan without building or syncing");
  io.print("  --check     probe the site DNS record and R2 bucket via cloudflare tooling");
  io.print("");
  io.print("Secrets:");
  io.print("  .local/secrets/ship.env        OPENWEB_SITE_* contract (see AGENTS.md)");
  io.print("  .local/secrets/cloudflare.env  CLOUDFLARE_* contract used by --check");
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

async function routes(): Promise<string[]> {
  const text = await Deno.readTextFile(table);
  const paths = [...text.matchAll(/path:\s*"([^"]+)"/g)].map((found) => found[1]);
  if (paths.length === 0) {
    io.fail(`ship: no routes found in ${table}`);
  }
  return paths;
}

function deep(paths: string[]): string[] {
  return paths.filter((route) => route !== "/");
}

type Sync = { note: string; args: string[] };

function commands(bucket: string, url: string): Sync[] {
  const endpoint = url.replace(/\/$/, "");
  return [
    {
      note: "hashed assets, immutable cache",
      args: [
        "--endpoint-url",
        endpoint,
        "s3",
        "sync",
        dist,
        `s3://${bucket}`,
        "--delete",
        "--exclude",
        "*.html",
        "--cache-control",
        "public, max-age=31536000, immutable",
        "--no-progress",
      ],
    },
    {
      note: "html shells, short cache",
      args: [
        "--endpoint-url",
        endpoint,
        "s3",
        "cp",
        dist,
        `s3://${bucket}`,
        "--recursive",
        "--exclude",
        "*",
        "--include",
        "*.html",
        "--content-type",
        "text/html; charset=utf-8",
        "--cache-control",
        "public, max-age=60, must-revalidate",
        "--no-progress",
      ],
    },
  ];
}

function show(args: string[]): string {
  return ["aws", ...args].map((arg) => (arg.includes(" ") ? `"${arg}"` : arg)).join(" ");
}

async function plan(): Promise<void> {
  const site = await secrets("ship.env");
  const empty = unfilled(site, keys);
  const fill = (key: string): string => (site[key] ? site[key] : `<${key}>`);
  const paths = await routes();
  io.print("==> ship plan (dry run)");
  io.print("");
  io.print("build:");
  io.print("  pnpm --filter @open-web/react build");
  io.print("");
  io.print("snapshot:");
  for (const route of deep(paths)) {
    io.print(`  ${dist}/index.html -> ${dist}${route}/index.html`);
  }
  io.print("");
  io.print("sync:");
  io.print(
    "  env: AWS_ACCESS_KEY_ID=<OPENWEB_SITE_S3_AK> AWS_SECRET_ACCESS_KEY=<OPENWEB_SITE_S3_SK>",
  );
  io.print("  env: AWS_DEFAULT_REGION=auto AWS_EC2_METADATA_DISABLED=true");
  for (const command of commands(fill("OPENWEB_SITE_S3_BUCKET"), fill("OPENWEB_SITE_S3_URL"))) {
    io.print(`  ${show(command.args)}`);
  }
  io.print("");
  io.print("verify:");
  io.print(`  https://${fill("OPENWEB_SITE_DOMAIN")}/`);
  const first = deep(paths)[0];
  if (first !== undefined) {
    io.print(`  https://${fill("OPENWEB_SITE_DOMAIN")}${first}`);
  }
  if (empty.length > 0) {
    io.print("");
    io.print(`unfilled in ${secretsDir()}/ship.env: ${empty.join(", ")}`);
  }
}

async function probe(url: string): Promise<void> {
  const response = await fetch(url, { headers: { "cache-control": "no-cache" } });
  await response.body?.cancel();
  if (response.status !== 200) {
    io.fail(`ship: expected 200 from ${url}, got ${response.status}`);
  }
  io.print(`  200 ${url}`);
}

async function ship(): Promise<void> {
  const site = await secrets("ship.env");
  const empty = unfilled(site, keys);
  if (empty.length > 0) {
    io.fail(`ship: unfilled in ${secretsDir()}/ship.env: ${empty.join(", ")}`);
  }
  const paths = await routes();
  io.print("==> build");
  await cmd.run("pnpm", ["--filter", "@open-web/react", "build"]);
  if (!(await fs.file.exists(`${dist}/index.html`))) {
    io.fail(`ship: build produced no ${dist}/index.html`);
  }
  io.print("==> snapshot");
  const shell = await Deno.readTextFile(`${dist}/index.html`);
  for (const route of deep(paths)) {
    await fs.file.writeText(`${dist}${route}/index.html`, shell);
    io.print(`  ${dist}${route}/index.html`);
  }
  io.print("==> sync");
  const auth = {
    AWS_ACCESS_KEY_ID: site.OPENWEB_SITE_S3_AK,
    AWS_SECRET_ACCESS_KEY: site.OPENWEB_SITE_S3_SK,
    AWS_DEFAULT_REGION: "auto",
    AWS_EC2_METADATA_DISABLED: "true",
  };
  for (const command of commands(site.OPENWEB_SITE_S3_BUCKET, site.OPENWEB_SITE_S3_URL)) {
    io.print(`  ${command.note}`);
    await cmd.run("aws", command.args, { env: auth });
  }
  io.print("==> verify");
  await probe(`https://${site.OPENWEB_SITE_DOMAIN}/`);
  const first = deep(paths)[0];
  if (first !== undefined) {
    await probe(`https://${site.OPENWEB_SITE_DOMAIN}${first}`);
  }
  io.print("ship: ok");
}

async function check(): Promise<void> {
  const site = await secrets("ship.env");
  const cloud = await secrets("cloudflare.env");
  const gate = unfilled(cloud, ["CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_TOKEN"]);
  if (gate.length > 0) {
    io.print(`check: skipped (unfilled in ${secretsDir()}/cloudflare.env: ${gate.join(", ")})`);
    return;
  }
  const name = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", name]);
  const id = json.get(zone, ".id");
  io.print(`zone: ${name} (${id})`);
  const domain = site.OPENWEB_SITE_DOMAIN ?? "";
  if (domain === "") {
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
      domain,
    ]);
    if (json.len(records) === 0) {
      io.fail(`check: no DNS record for ${domain} in zone ${name}`);
    }
    const record = json.get(records, "[0]");
    io.print(`dns: ${domain} ${json.get(record, ".type")} -> ${json.get(record, ".content")}`);
  }
  const bucket = site.OPENWEB_SITE_S3_BUCKET ?? "";
  if (bucket === "") {
    io.print("check: skipped bucket probe (unfilled in ship.env: OPENWEB_SITE_S3_BUCKET)");
  } else {
    const account = cloud.CLOUDFLARE_ACCOUNT_ID;
    const response = await runseal.text([
      "@tool",
      "cloudflare",
      "api",
      "request",
      "GET",
      `/accounts/${account}/r2/buckets/${bucket}`,
    ]);
    const found = json.get(response, ".result");
    io.print(`bucket: ${json.get(found, ".name")} (${json.get(found, ".location")})`);
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
