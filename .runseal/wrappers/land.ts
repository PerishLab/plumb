import {
  booleanOption,
  helpRequested,
  parseArgs as parseCliArgs,
  requireNoPositionals,
  stringOption,
} from "@/lib/cli.ts";
import { Forgejo, origin, type Pull, type Remote, token } from "@/lib/forgejo/forgejo.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const guardTimeoutMs = 240_000;
const pollRegisterMs = 5_000;
const pollPendingMs = 10_000;

type Options = {
  base: string;
  body: string;
  dryRun: boolean;
  deleteBranch: boolean;
};

function usage(): void {
  io.print("Usage: runseal :land [options]");
  io.print("");
  io.print("Land the current clean topic branch.");
  io.print("The branch is pushed, a PR is created or reused,");
  io.print("the PR is squash-merged, main is synced, and the topic branch is deleted.");
  io.print("");
  io.print("Options:");
  io.print("  --base <branch>    base branch (default: main)");
  io.print("  --body <body>      pull request body override");
  io.print("  --dry-run          print planned actions without changing git or the remote");
  io.print("  --no-delete        keep the topic branch after merge");
}

function parseArgs(args: string[]): Options & { help: boolean } {
  const parsed = parseCliArgs(args, {
    string: ["base", "body"],
    boolean: ["dry-run", "no-delete", "help", "h"],
  });
  requireNoPositionals(parsed, "land", { allowHelp: true });
  return {
    base: stringOption(parsed, "base", "main"),
    body: stringOption(parsed, "body"),
    dryRun: booleanOption(parsed, "dry-run"),
    deleteBranch: !booleanOption(parsed, "no-delete"),
    help: helpRequested(parsed),
  };
}

const options = parseArgs([...Deno.args]);
if (options.help) {
  usage();
  Deno.exit(0);
}

await cmd.run("git", ["--version"], { stdout: "null" });
const repo = await root();
const remote = await origin(repo);

const branch = await currentBranch(repo);
if (options.dryRun) {
  await ensureLandable(repo, options.base, branch, { fetch: false });
  printPlan(options, branch, remote);
  Deno.exit(0);
}

await ensureLandable(repo, options.base, branch, { fetch: true });
if (!await cmd.exists("tea")) {
  io.fail(`land: missing required tool: tea; run tea login add --url https://${remote.host}`);
}
await cmd.run("git", ["push", "-u", "origin", branch], { cwd: repo });
const head = await cmd.text("git", ["rev-parse", "HEAD"], { cwd: repo });

const api = new Forgejo(remote, await token(remote));
const pull = await findOrCreateForgejo(repo, api, options, branch);
io.print(pull.url);
await waitForGuard(api, pull, head);
await api.merge(pull.number, options.deleteBranch, head);
await cmd.run("git", ["checkout", options.base], { cwd: repo });
await cmd.run("git", ["pull", "--ff-only", "origin", options.base], { cwd: repo });
if (options.deleteBranch && await gitOk(repo, ["rev-parse", "--verify", `refs/heads/${branch}`])) {
  await cmd.run("git", ["branch", "-D", branch], { cwd: repo });
}

async function root(): Promise<string> {
  return await cmd.text("git", ["rev-parse", "--show-toplevel"]);
}

async function currentBranch(repo: string): Promise<string> {
  const branch = await cmd.text("git", ["branch", "--show-current"], { cwd: repo });
  if (branch === "") {
    io.fail("land: detached HEAD is not a landable topic branch");
  }
  return branch;
}

async function ensureLandable(
  repo: string,
  base: string,
  branch: string,
  options: { fetch: boolean },
): Promise<void> {
  if (branch === base || branch === "main" || branch === "master") {
    io.fail(`land: must run on a topic branch, not ${branch}`);
  }
  const dirty = await cmd.text("git", ["status", "--short"], { cwd: repo });
  if (dirty.trim() !== "") {
    io.fail("land: working tree must be clean; commit or discard changes first");
  }
  if (options.fetch) {
    await cmd.run("git", ["fetch", "origin", base], { cwd: repo });
  }
  const remoteBase = `origin/${base}`;
  if (!await gitOk(repo, ["rev-parse", "--verify", remoteBase])) {
    io.fail(`land: missing ${remoteBase}; fetch or check the base branch name`);
  }
  if (!await gitOk(repo, ["merge-base", "--is-ancestor", remoteBase, "HEAD"])) {
    io.fail(`land: current branch must contain latest ${remoteBase}; rebase onto ${base} first`);
  }
  const ahead = Number(
    await cmd.text("git", ["rev-list", "--count", `${remoteBase}..HEAD`], { cwd: repo }),
  );
  if (!Number.isFinite(ahead) || ahead <= 0) {
    io.fail(`land: current branch has no commits ahead of ${remoteBase}`);
  }
}

async function waitForGuard(api: Forgejo, pull: Pull, head: string): Promise<void> {
  io.print(`land: waiting for guard on ${head} (timeout ${guardTimeoutMs / 1000}s)`);
  const deadline = Date.now() + guardTimeoutMs;
  while (Date.now() < deadline) {
    const combined = await api.state(head);
    if (combined.count === 0) {
      await sleep(pollRegisterMs);
      continue;
    }
    if (combined.state === "success") {
      return;
    }
    if (combined.state !== "pending") {
      io.fail(`land: guard ${combined.state} on ${head}; PR #${pull.number} left open`);
    }
    await sleep(pollPendingMs);
  }
  io.fail(
    `land: guard still pending on ${head} after ${
      guardTimeoutMs / 1000
    }s; PR #${pull.number} left open`,
  );
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function gitOk(repo: string, args: string[]): Promise<boolean> {
  return await cmd.status("git", args, {
    cwd: repo,
    stdin: "null",
    stdout: "null",
    stderr: "null",
  }) === 0;
}

async function findOrCreateForgejo(
  repo: string,
  api: Forgejo,
  options: Options,
  branch: string,
): Promise<Pull> {
  const existing = await api.find(options.base, branch);
  if (existing !== null) {
    return existing;
  }
  const title = await deriveTitle(repo, options.base, branch);
  const body = options.body === "" ? await deriveBody(repo, options.base) : options.body;
  return await api.create(options.base, branch, title, body);
}

async function deriveTitle(repo: string, base: string, branch: string): Promise<string> {
  const subjects = await cmd.text("git", [
    "log",
    "--reverse",
    "--format=%s",
    `origin/${base}..HEAD`,
  ], { cwd: repo });
  const first = subjects.split(/\r?\n/).find((line) => line.trim() !== "");
  return first ?? branch;
}

async function deriveBody(repo: string, base: string): Promise<string> {
  return await cmd.text("git", ["log", "--reverse", "--format=%B", `origin/${base}..HEAD`], {
    cwd: repo,
  });
}

function printPlan(options: Options, branch: string, remote: Remote): void {
  const steps = [
    "[dry-run] would run:",
    `  git fetch origin ${options.base}`,
    `  verify ${branch} is clean, not ${options.base}, contains origin/${options.base}, ahead >= 1`,
    `  git push -u origin ${branch}`,
  ];
  const api = `https://${remote.host}/api/v1/repos/${remote.owner}/${remote.repo}`;
  steps.push(`  GET ${api}/pulls?state=open`);
  steps.push(`  POST ${api}/pulls  (base=${options.base}, head=${branch})  # if missing`);
  steps.push(`  GET ${api}/commits/<head-sha>/status  # poll until guard succeeds`);
  steps.push(
    `  POST ${api}/pulls/<n>/merge  (Do=squash, head_commit_id=<head-sha>${
      options.deleteBranch ? ", delete_branch_after_merge=true" : ""
    })`,
  );
  steps.push(`  git checkout ${options.base}`);
  steps.push(`  git pull --ff-only origin ${options.base}`);
  if (options.deleteBranch) {
    steps.push(`  git branch -D ${branch}  # if still present locally`);
  }
  io.print(steps.join("\n"));
}
