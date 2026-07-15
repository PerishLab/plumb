import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://runseal.perish.uk/manage.sh | sh`;

const profile = `[resources]
root = ".local"

[[injections]]
type = "env"

[injections.vars]
APP_DATA_DIR = "resource://data"`;

const session = `$ runseal @resolve resource://data
~/app/.local/data

$ runseal sh -c 'echo $APP_DATA_DIR'
~/app/.local/data`;

const sample = `$ runseal :guard
==> negentropy version pin
==> biome
==> tsc
==> vitest
==> deno fmt
==> deno check
==> negentropy
clean`;

export function Runseal() {
	return (
		<article>
			<Banner
				mark="/marks/runseal.svg"
				title="runseal"
				line="keep flows explicit."
			>
				<Badge>stable {releases.runseal}</Badge>
				<Forge repo="PerishCode/runseal" />
			</Banner>
			<p>
				operational glue rots: too many environment variables, too many
				machine-specific assumptions, too much of the flow living in shell
				history and uncontrolled script stacks. runseal gives the glue one small
				explicit profile — declared resources, named wrappers, injected env —
				without becoming a task runner or a secret manager.
			</p>
			<p>
				<Badge>env</Badge> <Badge>symlink</Badge> <Badge>argv</Badge>{" "}
				<Badge>deno</Badge>
			</p>
			<h2>quickstart</h2>
			<Code copy>{start}</Code>
			<p>
				manage.sh fetches the released binary from R2 into ~/.local/bin — linux,
				macos, and windows. then declare a profile at the repo root
				(runseal.toml) and every command you run through runseal sees it:
			</p>
			<Code>{profile}</Code>
			<Code>{session}</Code>
			<Grid>
				<Card title="routing">
					<p>
						the first token decides everything: a bare command runs inside the
						profile, a :name resolves a wrapper, and an @name runs a
						runseal-owned command like @profile or @resolve.
					</p>
				</Card>
				<Card title="resources">
					<p>
						<code>{"resource://ssh/config"}</code> is a profile-only path
						literal resolved to an absolute path under the declared root. child
						commands receive only the resolved path.
					</p>
				</Card>
				<Card title="wrappers">
					<p>
						operator flows are deno .ts wrappers under .runseal/wrappers,
						executed under a repo-declared permission policy: guard, land, init,
						release.
					</p>
				</Card>
				<Card title="forge tools">
					<p>
						@tool holds the atomic surface — forgejo, github, and cloudflare
						helpers — while wrappers bind repo-local policy and flow around
						those atoms.
					</p>
				</Card>
			</Grid>
			<p>
				the receipt below is real: every change to this site lands through these
				wrappers.
			</p>
			<Code>{sample}</Code>
		</article>
	);
}
