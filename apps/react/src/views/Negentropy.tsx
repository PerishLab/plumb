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
	let sum = 0;
	for (const plane of rows) {
		sum += gain(plane);
	}
	return sum;
}

function gain(plane: number[][]): number {
	let sum = 0;
	for (const row of plane) {
		for (const cell of row) {
			if (cell > 0) {
				sum += cell;
			}
		}
	}
	return sum;
}`;

const clean = `$ negentropy --strict .
clean`;

const lexicon = `[compound]
dataset = "the industry's own word; data-set reads worse"`;

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
				compromise compounds quietly. negentropy turns that rot into a bill: a
				structural checker that makes every violation either fixed or visibly
				owed, so meaning moves out of prose and into structure, tests, and named
				vocabulary.
			</p>
			<p>
				it is not a linter: lint rules check style inside one language's
				toolchain. negentropy checks structure across languages with one shared
				parser — no target compilers, no target toolchains.
			</p>
			<p>
				<Badge>rust</Badge> <Badge>typescript</Badge> <Badge>tsx</Badge>{" "}
				<Badge>scss</Badge> <Badge>markdown</Badge>
			</p>
			<h2 id="loop">the loop</h2>
			<p>
				the premise, demonstrated. an agent wrote two violations in eight lines
				— a compound name and a block one idea too deep:
			</p>
			<Code name="src/helper.ts">{fixture}</Code>
			<Code>{finding}</Code>
			<p>
				the checker billed it; the agent paid — not by clever chaining, but the
				way the law intends: extract the buried idea and name it. same
				algorithm, same passes, two flat shapes:
			</p>
			<Code name="src/helper.ts">{mended}</Code>
			<Code>{clean}</Code>
			<p>
				every transcript on this page is a real run. this loop — write, judge,
				restructure, clean — is what the workshop is for. and when the compound
				truly is the industry's word, the other exit is honest too: register it
				with a rationale in vocabulary.toml, and the dictionary delta rides the
				same PR.
			</p>
			<Code name="vocabulary.toml">{lexicon}</Code>
			<h2 id="quickstart">quickstart</h2>
			<Code name="install" copy>
				{start}
			</Code>
			<p>
				manage.sh is a short, readable script: it fetches one released binary
				into ~/.local/bin — linux x86_64 today, more targets as the runner pool
				grows. on an existing repo start with negentropy --debt . to see the
				bill before strict makes it fatal. laws and territories are declared per
				repo in negentropy.toml:
			</p>
			<Code name="negentropy.toml">{config}</Code>
			<h2 id="seats">where the law sits</h2>
			<p>
				an agent can read a document and still drift; only gates bind. the
				contract is one line — the actual first line an agent reads before
				touching this site's source — and three gates stand behind it:
			</p>
			<Code name="AGENTS.md">{contract}</Code>
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
						cannot be committed, so no dirty change slips into history by
						accident.
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
