import { cmd } from "@/lib/std/cmd.ts";
import { fs } from "@/lib/std/fs.ts";
import { path } from "@/lib/std/path.ts";

export type Remote = {
  host: string;
  owner: string;
  repo: string;
};

export type Pull = {
  number: number;
  url: string;
};

export type CommitState = {
  state: string;
  count: number;
};

export function parseRemote(raw: string): Remote {
  const url = raw.trim();
  let host = "";
  let found = "";
  const scp = url.match(/^[^/@]+@([^:/]+):(.+)$/);
  if (url.includes("://")) {
    const parsed = new URL(url);
    host = parsed.hostname;
    found = parsed.pathname;
  } else if (scp) {
    host = scp[1];
    found = scp[2];
  } else {
    throw new Error(`unrecognized remote url: ${JSON.stringify(raw)}`);
  }
  const parts = found.replace(/^\/+/, "").replace(/\.git$/, "").split("/").filter((part) =>
    part !== ""
  );
  if (parts.length < 2) {
    throw new Error(`cannot derive owner/repo from remote url: ${JSON.stringify(raw)}`);
  }
  const owner = parts[parts.length - 2];
  const repo = parts[parts.length - 1];
  if (host === "github.com" || host.endsWith(".github.com")) {
    throw new Error(
      "origin must point to Forgejo: ssh://git@git.perish.top/PerishFire/negentropy.git",
    );
  }
  return { host, owner, repo };
}

export async function origin(root: string): Promise<Remote> {
  const url = await cmd.text("git", ["remote", "get-url", "origin"], { cwd: root });
  if (url === "") {
    throw new Error("no origin remote configured");
  }
  return parseRemote(url);
}

export async function token(remote: Remote): Promise<string> {
  const inline = Deno.env.get("NEGENTROPY_FORGEJO_TOKEN");
  if (inline !== undefined && inline.trim() !== "") {
    return inline.trim();
  }
  const file = teaConfigPath();
  const logins = parseTeaConfig(await fs.file.readTextIfExists(file));
  const login = logins.find((entry) =>
    host(entry.url) === remote.host || entry.ssh_host === remote.host
  );
  const value = login?.token ?? "";
  if (value === "") {
    throw new Error(
      `forgejo token not found; run tea login add --name negentropy --url https://${remote.host} --token <token>`,
    );
  }
  return value;
}

export class Forgejo {
  constructor(private readonly remote: Remote, private readonly token: string) {}

  private base(): string {
    return `https://${this.remote.host}/api/v1/repos/${this.remote.owner}/${this.remote.repo}`;
  }

  private async request(
    method: string,
    route: string,
    body?: unknown,
  ): Promise<{ status: number; json: unknown }> {
    const response = await fetch(`${this.base()}${route}`, {
      method,
      headers: {
        "Authorization": `token ${this.token}`,
        "Accept": "application/json",
        ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const raw = await response.text();
    let json: unknown = null;
    if (raw !== "") {
      try {
        json = JSON.parse(raw);
      } catch {
        json = raw;
      }
    }
    return { status: response.status, json };
  }

  async find(base: string, head: string): Promise<Pull | null> {
    const { status, json } = await this.request("GET", "/pulls?state=open&limit=50");
    if (status !== 200 || !Array.isArray(json)) {
      throw new Error(`forgejo: listing pulls failed (${status}): ${describe(json)}`);
    }
    for (const pull of json) {
      if (!record(pull)) {
        continue;
      }
      const from = record(pull.head) ? pull.head.ref : undefined;
      const into = record(pull.base) ? pull.base.ref : undefined;
      if (from === head && into === base) {
        return { number: Number(pull.number), url: String(pull.html_url ?? "") };
      }
    }
    return null;
  }

  async create(base: string, head: string, title: string, body: string): Promise<Pull> {
    const { status, json } = await this.request("POST", "/pulls", {
      base,
      head,
      title,
      body,
    });
    if (status !== 201 || !record(json)) {
      throw new Error(`forgejo: creating pull failed (${status}): ${describe(json)}`);
    }
    return { number: Number(json.number), url: String(json.html_url ?? "") };
  }

  async state(sha: string): Promise<CommitState> {
    const { status, json } = await this.request("GET", `/commits/${sha}/status`);
    if (status !== 200 || !record(json)) {
      throw new Error(`forgejo: fetching status of ${sha} failed (${status}): ${describe(json)}`);
    }
    const statuses = Array.isArray(json.statuses) ? json.statuses : [];
    return { state: String(json.state ?? ""), count: statuses.length };
  }

  async merge(number: number, remove: boolean, head: string): Promise<void> {
    const body = { Do: "squash", delete_branch_after_merge: remove, head_commit_id: head };
    let last = "";
    for (let attempt = 0; attempt < 6; attempt += 1) {
      const { status, json } = await this.request("POST", `/pulls/${number}/merge`, body);
      if (status === 200 || status === 204) {
        return;
      }
      last = `(${status}): ${describe(json)}`;
      if (status !== 405 && status !== 409) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000 * (attempt + 1)));
    }
    throw new Error(`forgejo: merging pull #${number} failed ${last}`);
  }
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function describe(value: unknown): string {
  if (record(value) && typeof value.message === "string") {
    return value.message;
  }
  if (typeof value === "string") {
    return value;
  }
  return JSON.stringify(value);
}

type TeaLogin = {
  url: string;
  token: string;
  ssh_host: string;
};

function teaConfigPath(): string {
  const home = Deno.env.get("HOME");
  if (home === undefined || home === "") {
    throw new Error("HOME is required to locate tea config");
  }
  return path.join(home, ".tea/tea.yml");
}

function parseTeaConfig(text: string): TeaLogin[] {
  const entries: Array<Record<string, string>> = [];
  let current: Record<string, string> | null = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "" || line === "logins:" || line === "logins: []") {
      continue;
    }
    if (line.startsWith("- ")) {
      current = {};
      entries.push(current);
      readField(current, line.slice(2));
      continue;
    }
    if (current !== null) {
      readField(current, line);
    }
  }
  return entries.map((entry) => ({
    url: entry.url ?? "",
    token: entry.token ?? "",
    ssh_host: entry.ssh_host ?? "",
  }));
}

function readField(entry: Record<string, string>, line: string): void {
  const split = line.indexOf(":");
  if (split < 0) {
    return;
  }
  const key = line.slice(0, split).trim();
  if (!["url", "token", "ssh_host"].includes(key)) {
    return;
  }
  entry[key] = unquote(line.slice(split + 1).trim());
}

function unquote(value: string): string {
  if (
    value.length >= 2 &&
    ((value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'")))
  ) {
    return value.slice(1, -1);
  }
  return value;
}

function host(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    const split = url.indexOf("://");
    if (split < 0) {
      return url;
    }
    return url.slice(split + 3).split("/")[0].split(":")[0];
  }
}
