import { Badge, Hero, List, Search } from "@open-web/components";
import { useState } from "react";
import vocabulary from "../data/vocabulary.json";
import { type Entry, type Root, sift } from "../lib/sift";

function Rack(props: { root: Root }) {
	return (
		<section>
			<h3>
				<code>{props.root.root}</code>
			</h3>
			<List>
				{props.root.atoms.map((atom) => (
					<li key={atom.word}>
						{atom.word}
						<Badge>{atom.count}</Badge>
					</li>
				))}
			</List>
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
				text="the living dictionary: every atom declared across the four repos, grouped by module root and counted, baked straight out of negentropy --vocabulary."
			/>
			<Search value={query} change={update} hint="filter atoms" />
			{entries.map((entry) => (
				<Shelf key={entry.repo} entry={entry} />
			))}
		</article>
	);
}
