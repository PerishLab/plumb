import { cli, flags } from "@perish/sealkit/cli";
import { Forgejo, type Remote, token } from "@perish/sealkit/forgejo";
import { Project } from "@perish/sealkit/forgejo-project";
import { io } from "@perish/sealkit/io";
import { project } from "../lib/cold-start/project.ts";

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type RecordJson = { [key: string]: Json };

type Options = {
  product: string;
  repo: string;
  bucket: string;
  domain: string;
  zoneId: string;
  factoryEnv: string;
  output: string;
  forgejoUrl: string;
  dryRun: boolean;
};

type Factory = {
  account: string;
  token: string;
};

type Token = {
  id: string;
  name: string;
};

type Capability = {
  accessKey: string;
  secretKey: string;
  bucket: string;
  endpoint: string;
};

type Release = {
  publicUrl: string;
  publish: Capability;
  activate: Capability;
};

const API = Deno.env.get("CLOUDFLARE_API_BASE") ?? "https://api.cloudflare.com/client/v4";
const encoder = new TextEncoder();

function usage(): void {
  io.print("Usage: runseal :cold-start release [options]");
  io.print("");
  io.print("Cold-start one R2-backed Forgejo release delivery chain.");
  io.print("");
  io.print("Required:");
  io.print("  --product <name>       binary product, for example ectropy");
  io.print("  --repo <owner/name>    Forgejo repository");
  io.print("  --bucket <name>        exact R2 bucket name");
  io.print("  --domain <host>        exact public custom domain");
  io.print("  --zone-id <id>         Cloudflare zone containing the domain");
  io.print("");
  io.print("Optional:");
  io.print(
    "  --factory-env <path>  default: .local/secrets/cloudflare-token-factory.env",
  );
  io.print("  --output <path>       default: .local/secrets/releases/<product>.env");
  io.print("  --forgejo-url <url>   default: https://git.perish.top");
  io.print("  --dry-run             print the exact plan without reading credentials");
}

function parse(): Options {
  const args = cli.parse(Deno.args, {
    boolean: ["help", "h", "dry-run"],
    string: [
      "product",
      "repo",
      "bucket",
      "domain",
      "zone-id",
      "factory-env",
      "output",
      "forgejo-url",
    ],
  });
  const held = flags(args);
  if (held.help()) {
    usage();
    Deno.exit(0);
  }
  const action = held.single("cold-start");
  if (action !== "release") {
    io.fail(`cold-start: expected action release, got ${action === "" ? "<none>" : action}`);
  }
  const product = held.string("product");
  const repo = held.string("repo");
  const bucket = held.string("bucket");
  const domain = held.string("domain");
  const zoneId = held.string("zone-id");
  for (const [name, value] of Object.entries({ product, repo, bucket, domain, zoneId })) {
    if (value === "") {
      io.fail(
        `cold-start: --${name.replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`)} is required`,
      );
    }
  }
  if (!/^[a-z][a-z0-9-]*$/.test(product)) {
    io.fail("cold-start: --product must be lowercase kebab-case");
  }
  if (!/^[^/\s]+\/[^/\s]+$/.test(repo)) {
    io.fail("cold-start: --repo must be owner/name");
  }
  if (!/^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$/.test(bucket)) {
    io.fail("cold-start: --bucket is not a valid R2 bucket name");
  }
  if (!/^[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$/.test(domain) || !domain.includes(".")) {
    io.fail("cold-start: --domain is not a valid hostname");
  }
  if (!/^[a-f0-9]{32}$/.test(zoneId)) {
    io.fail("cold-start: --zone-id must be a 32-character lowercase hex ID");
  }
  return {
    product,
    repo,
    bucket,
    domain,
    zoneId,
    factoryEnv: held.string(
      "factory-env",
      ".local/secrets/cloudflare-token-factory.env",
    ),
    output: held.string("output", `.local/secrets/releases/${product}.env`),
    forgejoUrl: held.string("forgejo-url", "https://git.perish.top"),
    dryRun: held.boolean("dry-run"),
  };
}

async function readEnv(path: string): Promise<Record<string, string>> {
  const text = await Deno.readTextFile(path);
  const values: Record<string, string> = {};
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) {
      continue;
    }
    const at = line.indexOf("=");
    if (at < 1) {
      io.fail(`cold-start: invalid env line in ${path}`);
    }
    values[line.slice(0, at).trim()] = line.slice(at + 1).trim().replace(/^["']|["']$/g, "");
  }
  return values;
}

async function factory(path: string): Promise<Factory> {
  let values: Record<string, string>;
  try {
    values = await readEnv(path);
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      io.fail(`cold-start: missing credential file: ${path}`);
    }
    throw error;
  }
  const account = values.CLOUDFLARE_ACCOUNT_ID ?? "";
  const token = values.CLOUDFLARE_TOKEN_FACTORY_TOKEN ?? "";
  if (!/^[a-f0-9]{32}$/.test(account)) {
    io.fail(`cold-start: invalid CLOUDFLARE_ACCOUNT_ID in ${path}`);
  }
  if (token === "") {
    io.fail(`cold-start: CLOUDFLARE_TOKEN_FACTORY_TOKEN is empty in ${path}`);
  }
  return { account, token };
}

function record(value: Json | undefined): RecordJson {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("expected JSON object");
  }
  return value;
}

function string(value: Json | undefined, context: string): string {
  if (typeof value !== "string" || value === "") {
    throw new Error(`missing string: ${context}`);
  }
  return value;
}

function array(value: Json | undefined, context: string): Json[] {
  if (!Array.isArray(value)) {
    throw new Error(`missing array: ${context}`);
  }
  return value;
}

class HttpError extends Error {
  constructor(
    readonly statusCode: number,
    message: string,
  ) {
    super(message);
  }
}

async function api(
  token: string,
  method: string,
  path: string,
  body?: Json,
): Promise<Json> {
  const delays = [1000, 2000, 4000, 8000];
  for (let attempt = 0;; attempt += 1) {
    const response = await fetch(`${API}${path}`, {
      method,
      headers: {
        authorization: `Bearer ${token}`,
        ...(body === undefined ? {} : { "content-type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await response.text();
    let payload: Json;
    try {
      payload = JSON.parse(text) as Json;
    } catch {
      payload = text;
    }
    if (response.ok) {
      const envelope = record(payload);
      if (envelope.success !== true) {
        throw new Error(`Cloudflare ${method} ${path} answered without success`);
      }
      return envelope.result ?? null;
    }
    if ((response.status === 429 || response.status >= 500) && attempt < delays.length) {
      await new Promise((resolve) => setTimeout(resolve, delays[attempt]));
      continue;
    }
    const diagnostic = typeof payload === "string" ? payload : JSON.stringify(payload);
    throw new HttpError(
      response.status,
      `Cloudflare ${method} ${path} -> ${response.status}: ${diagnostic.slice(0, 300)}`,
    );
  }
}

async function tokens(keys: Factory): Promise<Token[]> {
  const found = array(
    await api(keys.token, "GET", `/accounts/${keys.account}/tokens?per_page=50`),
    "account tokens",
  );
  return found.map((entry) => {
    const held = record(entry);
    return { id: string(held.id, "token.id"), name: string(held.name, "token.name") };
  });
}

async function permission(
  keys: Factory,
  name: string,
  scope: string,
): Promise<string> {
  const found = array(
    await api(keys.token, "GET", `/accounts/${keys.account}/tokens/permission_groups`),
    "permission groups",
  ).filter((entry) => {
    const held = record(entry);
    return held.name === name && array(held.scopes, `${name}.scopes`).includes(scope);
  });
  if (found.length !== 1) {
    throw new Error(`expected exactly one Cloudflare permission group ${name} at ${scope}`);
  }
  return string(record(found[0]).id, `${name}.id`);
}

async function revoke(keys: Factory, id: string): Promise<void> {
  await api(keys.token, "DELETE", `/accounts/${keys.account}/tokens/${id}`);
}

async function createToken(
  keys: Factory,
  name: string,
  permissionId: string,
  resource: string,
  expiresOn?: string,
): Promise<{ id: string; value: string }> {
  const result = record(
    await api(keys.token, "POST", `/accounts/${keys.account}/tokens`, {
      name,
      policies: [{
        effect: "allow",
        resources: { [resource]: "*" },
        permission_groups: [{ id: permissionId }],
      }],
      ...(expiresOn === undefined ? {} : { expires_on: expiresOn }),
    }),
  );
  return {
    id: string(result.id, "created token id"),
    value: string(result.value, "created token value"),
  };
}

async function ensureBucket(admin: string, account: string, bucket: string): Promise<void> {
  try {
    await api(admin, "GET", `/accounts/${account}/r2/buckets/${bucket}`);
    io.print(`bucket: present (${bucket})`);
  } catch (error) {
    if (!(error instanceof HttpError) || error.statusCode !== 404) {
      throw error;
    }
    await api(admin, "POST", `/accounts/${account}/r2/buckets`, { name: bucket });
    io.print(`bucket: created (${bucket})`);
  }
}

async function ensureDomain(
  admin: string,
  account: string,
  bucket: string,
  domain: string,
  zoneId: string,
): Promise<void> {
  const path = `/accounts/${account}/r2/buckets/${bucket}/domains/custom`;
  const listed = record(await api(admin, "GET", path));
  const domains = array(listed.domains, "custom domains");
  const matches = domains.filter((entry) => record(entry).domain === domain);
  if (matches.length > 1) {
    throw new Error(`Cloudflare returned duplicate custom domains for ${domain}`);
  }
  if (matches.length === 0) {
    await api(admin, "POST", path, {
      domain,
      enabled: true,
      zoneId,
      minTLS: "1.2",
    });
    io.print(`domain: attached (${domain})`);
  } else {
    const held = record(matches[0]);
    if (held.zoneId !== zoneId) {
      throw new Error(`custom domain ${domain} belongs to unexpected zone ${held.zoneId}`);
    }
    if (held.enabled !== true || held.minTLS !== "1.2") {
      await api(admin, "PUT", `${path}/${domain}`, { enabled: true, minTLS: "1.2" });
      io.print(`domain: normalized (${domain}, TLS 1.2)`);
    } else {
      io.print(`domain: present (${domain})`);
    }
  }
}

async function waitDomain(
  admin: string,
  account: string,
  bucket: string,
  domain: string,
): Promise<void> {
  const path = `/accounts/${account}/r2/buckets/${bucket}/domains/custom/${domain}`;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    const held = record(await api(admin, "GET", path));
    const state = record(held.status);
    if (state.ownership === "active" && state.ssl === "active") {
      io.print("domain: active");
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  throw new Error(`custom domain ${domain} did not become active within 40 seconds`);
}

async function sha256(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(value));
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function releaseText(release: Release): string {
  return [
    `RELEASE_PUBLISH_S3_ACCESS_KEY=${release.publish.accessKey}`,
    `RELEASE_PUBLISH_S3_SECRET_KEY=${release.publish.secretKey}`,
    `RELEASE_PUBLISH_S3_BUCKET=${release.publish.bucket}`,
    `RELEASE_PUBLISH_S3_ENDPOINT=${release.publish.endpoint}`,
    `RELEASE_ACTIVATE_S3_ACCESS_KEY=${release.activate.accessKey}`,
    `RELEASE_ACTIVATE_S3_SECRET_KEY=${release.activate.secretKey}`,
    `RELEASE_ACTIVATE_S3_BUCKET=${release.activate.bucket}`,
    `RELEASE_ACTIVATE_S3_ENDPOINT=${release.activate.endpoint}`,
    "",
  ].join("\n");
}

async function writeAtomic(path: string, text: string): Promise<void> {
  const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  const parent = slash < 0 ? "." : path.slice(0, slash);
  await Deno.mkdir(parent, { recursive: true, mode: 0o700 });
  if (Deno.build.os !== "windows") {
    await Deno.chmod(parent, 0o700);
  }
  const draft = `${path}.tmp-${crypto.randomUUID()}`;
  try {
    await Deno.writeTextFile(draft, text, { mode: 0o600, createNew: true });
    if (Deno.build.os !== "windows") {
      await Deno.chmod(draft, 0o600);
    }
    await Deno.rename(draft, path);
  } finally {
    try {
      await Deno.remove(draft);
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) {
        throw error;
      }
    }
  }
}

async function verifyS3(name: string, capability: Capability): Promise<void> {
  const args = [
    "--endpoint-url",
    capability.endpoint,
    "s3api",
    "list-objects-v2",
    "--bucket",
    capability.bucket,
    "--max-keys",
    "1",
  ];
  const held = {
    AWS_ACCESS_KEY_ID: capability.accessKey,
    AWS_SECRET_ACCESS_KEY: capability.secretKey,
    AWS_DEFAULT_REGION: "auto",
  };
  const delays = [1000, 2000, 4000, 8000];
  for (let attempt = 0;; attempt += 1) {
    const output = await new Deno.Command("aws", {
      args,
      env: held,
      stdin: "null",
      stdout: "piped",
      stderr: "piped",
    }).output();
    if (output.success) {
      io.print(`${name} credential: ok`);
      return;
    }
    if (attempt < delays.length) {
      io.print(`${name} credential: waiting for propagation (${attempt + 1}/${delays.length})`);
      await new Promise((resolve) => setTimeout(resolve, delays[attempt]));
      continue;
    }
    const diagnostic = new TextDecoder().decode(output.stderr).trim();
    throw new Error(`aws failed (${output.code}): ${diagnostic}`);
  }
}

async function syncForgejo(options: Options, release: Release): Promise<void> {
  const remote = forgejo(options);
  const api = new Forgejo(remote, await token(remote));
  const project = new Project(api);
  const values: Record<string, string> = {
    RELEASE_PUBLISH_S3_ACCESS_KEY: release.publish.accessKey,
    RELEASE_PUBLISH_S3_SECRET_KEY: release.publish.secretKey,
    RELEASE_PUBLISH_S3_BUCKET: release.publish.bucket,
    RELEASE_PUBLISH_S3_ENDPOINT: release.publish.endpoint,
    RELEASE_ACTIVATE_S3_ACCESS_KEY: release.activate.accessKey,
    RELEASE_ACTIVATE_S3_SECRET_KEY: release.activate.secretKey,
    RELEASE_ACTIVATE_S3_BUCKET: release.activate.bucket,
    RELEASE_ACTIVATE_S3_ENDPOINT: release.activate.endpoint,
  };
  for (
    const name of [
      "RELEASE_PUBLISH_S3_ACCESS_KEY",
      "RELEASE_PUBLISH_S3_SECRET_KEY",
      "RELEASE_PUBLISH_S3_BUCKET",
      "RELEASE_PUBLISH_S3_ENDPOINT",
      "RELEASE_ACTIVATE_S3_ACCESS_KEY",
      "RELEASE_ACTIVATE_S3_SECRET_KEY",
      "RELEASE_ACTIVATE_S3_BUCKET",
      "RELEASE_ACTIVATE_S3_ENDPOINT",
    ]
  ) {
    await project.secret(name, values[name]);
    io.print(`forgejo secret: upserted (${name})`);
  }
  io.print(`forgejo: synced (${options.repo})`);
}

function forgejo(options: Options): Remote {
  const url = new URL(options.forgejoUrl);
  const names = options.repo.split("/");
  return {
    scheme: url.protocol.slice(0, -1),
    host: url.port === "" ? url.hostname : `${url.hostname}:${url.port}`,
    owner: names[0],
    repo: names[1],
  };
}

async function existingRelease(
  options: Options,
  publisher: Token | undefined,
  activator: Token | undefined,
): Promise<Release | undefined> {
  let values: Record<string, string>;
  try {
    values = await readEnv(options.output);
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      values = {};
    } else {
      throw error;
    }
  }
  const names = [
    "RELEASE_PUBLISH_S3_ACCESS_KEY",
    "RELEASE_PUBLISH_S3_SECRET_KEY",
    "RELEASE_PUBLISH_S3_BUCKET",
    "RELEASE_PUBLISH_S3_ENDPOINT",
    "RELEASE_ACTIVATE_S3_ACCESS_KEY",
    "RELEASE_ACTIVATE_S3_SECRET_KEY",
    "RELEASE_ACTIVATE_S3_BUCKET",
    "RELEASE_ACTIVATE_S3_ENDPOINT",
  ];
  const present = names.filter((name) => (values[name] ?? "") !== "");
  if (present.length === 0 && publisher === undefined && activator === undefined) {
    return undefined;
  }
  if (present.length !== names.length) {
    throw new Error(`cold-start: incomplete release escrow at ${options.output}`);
  }
  if (publisher === undefined || activator === undefined) {
    throw new Error(`cold-start: release escrow and persistent capabilities disagree`);
  }
  if (
    values.RELEASE_PUBLISH_S3_ACCESS_KEY !== publisher.id ||
    values.RELEASE_ACTIVATE_S3_ACCESS_KEY !== activator.id
  ) {
    throw new Error(`cold-start: release escrow does not match persistent capabilities`);
  }
  return {
    publicUrl: `https://${options.domain}`,
    publish: {
      accessKey: values.RELEASE_PUBLISH_S3_ACCESS_KEY,
      secretKey: values.RELEASE_PUBLISH_S3_SECRET_KEY,
      bucket: values.RELEASE_PUBLISH_S3_BUCKET,
      endpoint: values.RELEASE_PUBLISH_S3_ENDPOINT,
    },
    activate: {
      accessKey: values.RELEASE_ACTIVATE_S3_ACCESS_KEY,
      secretKey: values.RELEASE_ACTIVATE_S3_SECRET_KEY,
      bucket: values.RELEASE_ACTIVATE_S3_BUCKET,
      endpoint: values.RELEASE_ACTIVATE_S3_ENDPOINT,
    },
  };
}

async function release(): Promise<void> {
  const options = parse();
  const tempName = `tmp:${options.bucket}`;
  const publishName = `publish:${options.bucket}`;
  const activateName = `activate:${options.bucket}`;
  io.print("==> release cold-start");
  io.print(`product: ${options.product}`);
  io.print(`repo: ${options.repo}`);
  io.print(`bucket: ${options.bucket}`);
  io.print(`domain: ${options.domain}`);
  io.print(`factory token: super:perish.code (${options.factoryEnv})`);
  io.print(`temporary token: ${tempName}`);
  io.print(`publish capability: ${publishName}`);
  io.print(`activate capability: ${activateName}`);
  io.print(`escrow: ${options.output}`);
  if (options.dryRun) {
    io.print("dry-run: no credentials read and no state changed");
    return;
  }

  const keys = await factory(options.factoryEnv);
  await api(keys.token, "GET", `/accounts/${keys.account}/tokens/verify`);
  const all = await tokens(keys);
  for (const stale of all.filter((token) => token.name === tempName)) {
    await revoke(keys, stale.id);
    io.print(`temporary token: cleared stale ${tempName}`);
  }
  const publishers = all.filter((token) => token.name === publishName);
  const activators = all.filter((token) => token.name === activateName);
  if (publishers.length > 1 || activators.length > 1) {
    throw new Error(`cold-start: duplicate persistent capability name`);
  }
  const existing = await existingRelease(options, publishers[0], activators[0]);
  if ((publishers.length === 1 || activators.length === 1) && existing === undefined) {
    throw new Error(
      `cold-start: persistent capability exists but ${options.output} cannot recover its one-time secret`,
    );
  }

  const adminPermission = await permission(
    keys,
    "Workers R2 Storage Write",
    "com.cloudflare.api.account",
  );
  const objectPermission = await permission(
    keys,
    "Workers R2 Storage Bucket Item Write",
    "com.cloudflare.edge.r2.bucket",
  );
  const expires = new Date(Date.now() + 15 * 60 * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");
  const temporary = await createToken(
    keys,
    tempName,
    adminPermission,
    `com.cloudflare.api.account.${keys.account}`,
    expires,
  );
  const created: string[] = [];
  let escrowed = existing !== undefined;
  try {
    io.print(`temporary token: active until ${expires}`);
    await ensureBucket(temporary.value, keys.account, options.bucket);
    await ensureDomain(
      temporary.value,
      keys.account,
      options.bucket,
      options.domain,
      options.zoneId,
    );

    let release = existing;
    if (release === undefined) {
      const publisher = await createToken(
        keys,
        publishName,
        objectPermission,
        `com.cloudflare.edge.r2.bucket.${keys.account}_default_${options.bucket}`,
      );
      created.push(publisher.id);
      const activator = await createToken(
        keys,
        activateName,
        objectPermission,
        `com.cloudflare.edge.r2.bucket.${keys.account}_default_${options.bucket}`,
      );
      created.push(activator.id);
      const endpoint = `https://${keys.account}.r2.cloudflarestorage.com`;
      release = {
        publicUrl: `https://${options.domain}`,
        publish: {
          accessKey: publisher.id,
          secretKey: await sha256(publisher.value),
          bucket: options.bucket,
          endpoint,
        },
        activate: {
          accessKey: activator.id,
          secretKey: await sha256(activator.value),
          bucket: options.bucket,
          endpoint,
        },
      };
      await writeAtomic(options.output, releaseText(release));
      escrowed = true;
      io.print(`release escrow: written (${options.output})`);
    } else {
      io.print(`release escrow: resumed (${options.output})`);
    }
    await verifyS3("publish", release.publish);
    await verifyS3("activate", release.activate);
    await syncForgejo(options, release);
    await waitDomain(
      temporary.value,
      keys.account,
      options.bucket,
      options.domain,
    );
    io.print("cold-start: ok");
  } catch (error) {
    if (!escrowed) {
      for (const id of created) {
        await revoke(keys, id);
      }
    }
    throw error;
  } finally {
    await revoke(keys, temporary.id);
    io.print(`temporary token: revoked (${tempName})`);
  }
}

async function main(): Promise<void> {
  if (Deno.args[0] === "project") {
    await project(Deno.args.slice(1));
    return;
  }
  await release();
}

await main().catch((error) => {
  io.error(error instanceof Error ? error.message : String(error));
  Deno.exit(1);
});
