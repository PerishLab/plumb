import { Card, Grid, Hero, Ledger, Search } from "@open-web/components";
import { useState } from "react";
import vocabulary from "../data/vocabulary.json";
import { type Entry, type Root, sift } from "../lib/sift";

function Rack(props: { root: Root }) {
	return (
		<section>
			<h3>
				<code>{props.root.root}</code>
			</h3>
			<Ledger atoms={props.root.atoms} />
		</section>
	);
}

function Shelf(props: { entry: Entry }) {
	return (
		<section>
			<h2>{props.entry.repo}</h2>
			{props.entry.roots.map((root) => (
				<Rack key={root.root} root={root} />
			))}
		</section>
	);
}

export function Vocabulary() {
	const [query, update] = useState("");
	const entries = sift(vocabulary, query);
	return (
		<article>
			<Hero
				title="vocabulary"
				text="the living dictionary: every atom declared across the three tools and this site, grouped by module root, counted on every bake, and judged by case law."
			/>
			<h2>case law</h2>
			<p>
				single word only works against a living dictionary: agents maintain
				vocabulary deltas alongside code diffs, a registered compound demands a
				rationale, and a contested word gets a verdict. three rulings from the
				book:
			</p>
			<Grid>
				<Card title="kill">
					<p>
						kill is force. it never carries a signal parameter; the polite path
						is stop.
					</p>
				</Card>
				<Card title="guard">
					<p>
						one gate, three faces: guard.yml in CI, runseal :guard locally, and
						the guard verb. any composite pass-before-proceed barrier is guard;
						nothing else may take the name.
					</p>
				</Card>
				<Card title="land">
					<p>
						land is the only verb allowed to end at the mainline; everything
						else stops at the PR.
					</p>
				</Card>
			</Grid>
			<h2>the atoms</h2>
			<Search value={query} change={update} hint="filter atoms" />
			{entries.map((entry) => (
				<Shelf key={entry.repo} entry={entry} />
			))}
		</article>
	);
}
