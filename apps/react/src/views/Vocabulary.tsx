import { Card, Grid, Hero, Ledger, List, Search } from "@open-web/components";
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
			<h2 id={props.entry.repo}>{props.entry.repo}</h2>
			{props.entry.roots.map((root) => (
				<Rack key={root.root} root={root} />
			))}
		</section>
	);
}

export function Jump(props: { entries: Entry[] }) {
	return (
		<List>
			{props.entries.map((entry) => (
				<li key={entry.repo}>
					<a href={`#${entry.repo}`}>{entry.repo}</a>
				</li>
			))}
		</List>
	);
}

export function Vocabulary() {
	const [query, update] = useState("");
	const entries = sift(vocabulary, query);
	return (
		<article>
			<Hero
				title="vocabulary"
				text="the living dictionary: every name declared across the three tools and this site, grouped by the folder that owns it and recounted on every site build. contested words get written verdicts below."
			/>
			<h2 id="rulings">case law</h2>
			<p>
				the single-word rule only works if the dictionary lives: when an agent
				needs a new word, the dictionary change rides the same PR as the code
				that needs it; a compound name must argue its case in writing; and a
				contested word gets a ruling. three from the book:
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
			<h2 id="atoms">the atoms</h2>
			<Jump entries={entries} />
			<Search value={query} change={update} hint="filter atoms" />
			{entries.length === 0 ? <p>no atoms match "{query}".</p> : null}
			{entries.map((entry) => (
				<Shelf key={entry.repo} entry={entry} />
			))}
		</article>
	);
}
