import { Badge, Card, Grid, Hero } from "@open-web/components";

const laws = [
	{
		name: "word",
		kind: "debt",
		text: "a declared name is one vocabulary atom; an unregistered compound is debt.",
		why: "single-word pressure points downward to the language namespace, and upward to a living vocabulary that gives the atom meaning.",
	},
	{
		name: "path",
		kind: "fault",
		text: "directory nesting past four levels below the owning module root is a fault.",
		why: "the block signal at the file-tree scale. a path is a name; keep it a short one.",
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
		why: "a comment is explanation that failed to become structure — and an agent will not reread it next session. denial moves the pressure into a better name, a test, a vocabulary entry, or a doc. the escape hatch is declared, not implied: a boundary entry in negentropy.toml can allow comments where an external quirk truly demands them — priced and visible.",
	},
	{
		name: "grant",
		kind: "fault",
		text: "granted syntax outside its declared territory is a fault; test and style are the granted classes.",
		why: "with a grant declared, product files hold zero test code — ls is the audit. no grant declared means the class is unrestricted.",
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
				text="nine laws judge a codebase. they are written for repos maintained by AI agents, where explanation left in prose is explanation that dies: a fault fails the run, debt is reported and tolerated, and a blindspot is the scanner's honesty about an unparsed region."
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
				stable is a promise: the promised surfaces hold, and the guard's version
				policy prices every change — an unchanged kernel keeps to patch bumps, a
				changed kernel pays a minor or higher.
			</p>
		</article>
	);
}
