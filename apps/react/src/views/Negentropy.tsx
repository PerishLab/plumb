import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://releases.negentropy.perish.uk/manage.sh | sh
negentropy --strict .`;

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
				x86_64 today, more targets as the runner pool grows. laws and
				territories are declared per repo in negentropy.toml; without one, the
				defaults judge the whole tree.
			</p>
			<h2>a first finding</h2>
			<p>
				two violations in eight lines: a compound name and a block one idea too
				deep.
			</p>
			<Code>{fixture}</Code>
			<Code>{finding}</Code>
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
