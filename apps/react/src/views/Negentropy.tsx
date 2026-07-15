import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://releases.negentropy.perish.uk/manage.sh | sh
negentropy --strict .`;

const config = `[scan]
include = ["src/**/*.ts"]

[module]
roots = ["src"]

[limit]
block = 4
path = 4

[comment]
allow = false

[word]
single = true`;

const fixture = `export function fetchAllData(rows: number[][][]): number {
	let sum = 0;
	for (const plane of rows) {
		for (const row of plane) {
			for (const cell of row) {
				if (cell > 0) {
					sum += cell;`;

const finding = `$ negentropy --debt .
src/helper.ts:6:19 block depth over limit
src/helper.ts:1:17 debt fetchAllData
1 faults, 0 blindspots, 1 debt
by law: block=1 word=1
hot files: src/helper.ts=2`;

const mended = `export function tally(rows: number[][][]): number {
	const cells = rows.flat(2);
	return cells.filter((cell) => cell > 0).reduce((sum, cell) => sum + cell, 0);
}`;

const clean = `$ negentropy --strict .
clean`;

const contract = `# AGENTS.md
This repository is a constitution-era web workspace.
\`negentropy --strict .\` must print \`clean\` before anything lands.`;

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
				an agent-maintained codebase rots differently: every session reads the
				code with fresh eyes, prose comments drift unread, and each small
				compromise compounds quietly. negentropy prices that entropy — a
				self-contained structural checker that pushes explanation pressure out
				of prose and into structure, tests, vocabulary, and docs.
			</p>
			<p>
				it is not a linter: lint rules police style inside one language's
				toolchain. negentropy's nine laws police structure across languages from
				one grammar-first parser substrate — no target compilers, no target
				toolchains.
			</p>
			<p>
				<Badge>rust</Badge> <Badge>typescript</Badge> <Badge>tsx</Badge>{" "}
				<Badge>scss</Badge> <Badge>markdown</Badge>
			</p>
			<h2>quickstart</h2>
			<Code copy>{start}</Code>
			<p>
				manage.sh fetches the released binary from R2 into ~/.local/bin — linux
				x86_64 today, more targets as the runner pool grows. on an existing repo
				start with negentropy --debt . to see the bill before strict makes it
				fatal. laws and territories are declared per repo in negentropy.toml:
			</p>
			<Code>{config}</Code>
			<h2>the loop</h2>
			<p>
				the premise, demonstrated. an agent wrote two violations in eight lines
				— a compound name and a block one idea too deep:
			</p>
			<Code>{fixture}</Code>
			<Code>{finding}</Code>
			<p>
				the checker priced the entropy; the agent paid it down — a single word
				that resolves, a shape that is flat:
			</p>
			<Code>{mended}</Code>
			<Code>{clean}</Code>
			<p>
				every transcript on this page is a real run. this loop — write, judge,
				restructure, clean — is what the workshop is for.
			</p>
			<h2>where the law sits</h2>
			<p>
				an agent does not obey documents; it obeys gates. the law binds at four
				seats, and the contract is one line — this is the actual first line an
				agent reads before touching this site's source:
			</p>
			<Code>{contract}</Code>
			<Grid>
				<Card title="session">
					<p>
						AGENTS.md names the constitution, so every session starts under it.
						the agent reads the bill as it works: negentropy --debt . is cheap
						enough to run after every edit.
					</p>
				</Card>
				<Card title="commit">
					<p>
						the pre-commit hook runs the full guard: a tree that is not clean
						cannot be committed, so a session cannot end dirty by accident.
					</p>
				</Card>
				<Card title="merge">
					<p>
						guard.yml runs the same gauntlet in CI, and the land flow merges
						only on green. one gate, three faces — session, commit, merge.
					</p>
				</Card>
				<Card title="vocabulary">
					<p>
						when a name wants two words, the agent restructures — or registers
						the compound with a rationale. the dictionary delta rides the same
						PR as the code that needed it.
					</p>
				</Card>
			</Grid>
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
		</article>
	);
}
