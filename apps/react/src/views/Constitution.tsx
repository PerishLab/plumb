import { Badge, Card, Grid, Hero } from "@perish/react-components";

const laws = [
	{
		name: "word",
		kind: "debt",
		text: "a declared name is one vocabulary atom; an unregistered compound is debt.",
		why: "single-word pressure pushes down into the language namespace, and up into a living vocabulary that gives each atom meaning.",
	},
	{
		name: "path",
		kind: "fault",
		text: "directory nesting past four levels below the owning module root is a fault.",
		why: "the block signal at the file-tree scale. a path is a name; keep it a short one.",
	},
	{
		name: "file",
		kind: "fault",
		text: "a file running past three hundred lines is a fault.",
		why: "the fix is a move, not a squeeze — fmt owns line layout. a long file is more than one seat wearing one name.",
	},
	{
		name: "fanout",
		kind: "fault",
		text: "a directory holding more than ten children is a fault.",
		why: "a wide directory is a chapter refusing to split. the count sees only the scan set, so the width it reports only ever understates.",
	},
	{
		name: "block",
		kind: "fault",
		text: "scope nesting past four is a fault.",
		why: "deeper nesting is a unit carrying more than one idea. extract until the shape is flat.",
	},
	{
		name: "markup",
		kind: "fault",
		text: "element nesting past eight in one element tree is a fault; markup is its own axis, neither scope nor literal.",
		why: "deep markup is an atomic component refusing to exist: extract and name the wrapper instead of indenting further.",
	},
	{
		name: "comment",
		kind: "fault",
		text: "comments are denied by default; each one is a fault.",
		why: "a comment is an explanation that failed to become structure — and no gate makes the next session reread it. denial moves the pressure into a better name, a test, a vocabulary entry, or a doc. the escape hatch is declared, not implied: a boundary entry in negentropy.toml can allow comments where an external quirk truly demands them — priced and visible.",
	},
	{
		name: "grant",
		kind: "fault",
		text: "granted syntax outside its declared territory is a fault; test and style are the granted classes.",
		why: "a grant confines a syntax class to its declared territory — tests in test paths, styles in the styled package. no grant declared means the class roams free.",
	},
	{
		name: "dispatch",
		kind: "debt",
		text: "a repeated equality subject across adjacent branches is a table refusing to exist.",
		why: "a decision is a thing; give it one node. guard chains over different subjects stay untouched.",
	},
	{
		name: "receiver",
		kind: "debt",
		text: "a fourth free function in one file on one receiver names an object that does not exist yet.",
		why: "build the object once and let methods read what they share. methods are exempt — they have declared their subject.",
	},
	{
		name: "param",
		kind: "debt",
		text: "a function holding more than four parameters, receiver included, is a struct refusing a name.",
		why: "name the bundle and pass it whole.",
	},
];

export function Constitution() {
	return (
		<article>
			<Hero
				title="constitution"
				text="eleven laws judge a codebase, and each one turns a kind of structural drift into something a gate can refuse. a violation lands in one of three classes — a fault fails the run, debt is reported and tolerated, and a blindspot marks a region the scanner could not parse. honesty, not silence."
			/>
			<Grid>
				{laws.map((law) => (
					<Card key={law.name} title={law.name}>
						<p>{law.text}</p>
						<p>{law.why}</p>
						<Badge>{law.kind}</Badge>
					</Card>
				))}
			</Grid>
			<p>
				stable is a promise: patch releases keep the checker's contract intact,
				and changing the contract costs a minor release or higher — the version
				policy makes every rule change pay its price in the open.
			</p>
		</article>
	);
}
