import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://releases.negentropy.perish.uk/manage.sh | sh
negentropy --strict .`;

const sample = `$ negentropy --debt .
src/deeply.ts:5:12 block depth over limit
src/helper.ts:1:17 debt fetchAllData
1 faults, 0 blindspots, 1 debt
by law: block=1 word=1
hot files: src/deeply.ts=1 src/helper.ts=1`;

export function Negentropy() {
	return (
		<article>
			<Banner
				mark="/marks/negentropy.svg"
				title="negentropy"
				line="keep entropy down."
			>
				<Badge>stable {releases.negentropy}</Badge>
				<Forge repo="PerishCode/negentropy" />
			</Banner>
			<p>
				a self-contained structural checker for reducing semantic entropy in
				codebases, especially agent-maintained ones. it pushes explanation
				pressure out of prose and into structure, tests, vocabulary, and docs.
			</p>
			<p>
				<Badge>rust</Badge> <Badge>typescript</Badge> <Badge>tsx</Badge>{" "}
				<Badge>scss</Badge> <Badge>markdown</Badge>
			</p>
			<h2>quickstart</h2>
			<Code copy>{start}</Code>
			<Grid>
				<Card title="grammar first">
					<p>
						a parser substrate with embedded g4 grammars delivers one uniform
						structure tree to language-agnostic checks. no target compilers, no
						target toolchains.
					</p>
				</Card>
				<Card title="nine laws">
					<p>
						word, path, block, markup, comment, grant, dispatch, receiver, param
						— each declared in negentropy.toml and judged on every run of the
						scanned tree.
					</p>
				</Card>
				<Card title="three classes">
					<p>
						a fault fails the run, debt is reported and tolerated, and a
						blindspot is the scanner being honest about an unparsed region —
						fatal only under strict.
					</p>
				</Card>
				<Card title="living vocabulary">
					<p>
						single word only works against a living dictionary: agents maintain
						vocabulary deltas alongside code diffs, and registered compounds
						demand a rationale.
					</p>
				</Card>
			</Grid>
			<Code>{sample}</Code>
		</article>
	);
}
