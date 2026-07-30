import { cli, flags } from "@perish/sealkit/cli";
import { Forgejo, type Remote, token } from "@perish/sealkit/forgejo";
import { Project } from "@perish/sealkit/forgejo-project";
import { io } from "@perish/sealkit/io";

type Options = {
  repo: string;
  description: string;
  forgejoUrl: string;
  branch: string;
  private: boolean;
  dryRun: boolean;
};

export async function project(raw: string[]): Promise<void> {
  const options = parse(raw);
  io.print("==> project cold-start");
  io.print(`repo: ${options.repo}`);
  io.print(`visibility: ${options.private ? "private" : "public"}`);
  io.print(`integration branch: ${options.branch}`);
  io.print(`description: ${options.description}`);
  if (options.dryRun) {
    io.print("dry-run: no credentials read and no state changed");
    return;
  }
  const remote = forgejo(options);
  const api = new Forgejo(remote, await token(remote));
  const result = await new Project(api).ensure({
    description: options.description,
    private: options.private,
  });
  io.print(`forgejo repository: ${result.state} (${options.repo})`);
  io.print(`ssh: ${result.sshUrl}`);
}

function parse(raw: string[]): Options {
  const args = cli.parse(raw, {
    boolean: ["help", "h", "private", "dry-run"],
    string: ["repo", "description", "forgejo-url", "branch"],
  });
  const held = flags(args);
  held.positionals("project", { allowHelp: true });
  if (held.help()) {
    usage();
    Deno.exit(0);
  }
  const repo = held.string("repo");
  const description = held.string("description");
  const forgejoUrl = held.string("forgejo-url", "https://git.perish.top");
  const branch = held.string("branch", "main");
  if (!/^[^/\s]+\/[^/\s]+$/.test(repo)) {
    io.fail("cold-start project: --repo must be owner/name");
  }
  if (description === "") {
    io.fail("cold-start project: --description is required");
  }
  if (!/^[A-Za-z0-9._/-]+$/.test(branch) || branch.startsWith("-")) {
    io.fail("cold-start project: invalid --branch");
  }
  const url = new URL(forgejoUrl);
  if (!["http:", "https:"].includes(url.protocol) || url.pathname !== "/") {
    io.fail("cold-start project: --forgejo-url must be an HTTP(S) origin");
  }
  return {
    repo,
    description,
    forgejoUrl,
    branch,
    private: held.boolean("private"),
    dryRun: held.boolean("dry-run"),
  };
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

function usage(): void {
  io.print("Usage: runseal :cold-start project [options]");
  io.print("");
  io.print("Create or verify one exact empty Forgejo repository.");
  io.print("");
  io.print("Required:");
  io.print("  --repo <owner/name>");
  io.print("  --description <text>");
  io.print("");
  io.print("Optional:");
  io.print("  --forgejo-url <url>   default: https://git.perish.top");
  io.print("  --branch <name>       default: main");
  io.print("  --private             require a private repository");
  io.print("  --dry-run             print the plan without reading credentials");
}
