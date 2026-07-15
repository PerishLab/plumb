import { Badge, Card, Grid, Hero } from "@open-web/components";

const laws = [
	{
		name: "word",
		kind: "debt",
		text: "a declared name is one vocabulary atom; an unregistered compound is debt.",
	},
	{
		name: "path",
		kind: "fault",
		text: "directory nesting past four levels below the owning module root is a fault.",
	},
	{
		name: "block",
		kind: "fault",
		text: "scope nesting past four is a fault.",
	},
	{
		name: "comment",
		kind: "fault",
		text: "comments are denied by default; each one is a fault.",
	},
	{
		name: "grant",
		kind: "fault",
		text: "granted syntax outside its declared territory is a fault; test syntax is the first class.",
	},
	{
		name: "dispatch",
		kind: "debt",
		text: "a repeated equality subject across adjacent branches is a table refusing to exist.",
	},
	{
		name: "receiver",
		kind: "debt",
		text: "a fourth free function in one file on one receiver names an object that does not exist yet.",
	},
	{
		name: "param",
		kind: "debt",
		text: "a function holding more than four parameters, receiver included, is a struct refusing a name.",
	},
];

export function Constitution() {
	return (
		<article>
			<Hero
				title="constitution"
				text="eight laws judge a codebase, each landing in one of three classes: a fault fails the run, debt is reported and tolerated, and a blindspot marks an unparsed region."
			/>
			<Grid>
				{laws.map((law) => (
					<Card key={law.name} title={law.name}>
						<p>{law.text}</p>
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
