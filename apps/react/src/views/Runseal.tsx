import { Badge, Card, Code, Grid, Hero } from "@open-web/components";

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
			<Hero
				title="runseal"
				text="run a command inside a small, explicit profile. runseal gives a repo named wrappers, local resources, and env, argv, and symlink setup without becoming a task runner or a secret manager."
			>
				<Badge>env</Badge> <Badge>symlink</Badge> <Badge>argv</Badge>{" "}
				<Badge>deno</Badge>
			</Hero>
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
			<Code>{sample}</Code>
		</article>
	);
}
