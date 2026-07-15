import { Card, Grid, Hero } from "@open-web/components";
import { Link } from "react-router";
import { title } from "../lib/title";

export function Home() {
	return (
		<article>
			<Hero
				mark="/favicon.svg"
				title={title()}
				text="a workshop for code that AI agents maintain. the problem is familiar: an agent rewrites the file, the comment above it quietly becomes a lie, and nobody — human or agent — rereads it. so the rules here live where a checker can enforce them: in structure and named vocabulary, not in prose."
			/>
			<Grid>
				<Card title="negentropy">
					<p>
						keep entropy down: a grammar-first structural checker enforcing a
						nine-law constitution — single-word names, shallow blocks and paths,
						denied comments, granted syntax, and a vocabulary that never
						freezes.
					</p>
					<Link to="/negentropy">read the checker</Link>
				</Card>
				<Card title="runseal">
					<p>
						keep flows explicit: an operator toolbelt — guard, land, and release
						flows plus forge tools, all run inside a small explicit profile with
						named wrappers and declared resources.
					</p>
					<Link to="/runseal">open the toolbelt</Link>
				</Card>
				<Card title="sidecar">
					<p>
						keep local runtimes named: a manifest-driven process manager — one
						plan from one manifest, stamped process identity, leased ports, a
						namespace broker, and an inspect bridge.
					</p>
					<Link to="/sidecar">meet the manager</Link>
				</Card>
			</Grid>
			<h2>adopt in order of pain</h2>
			<p>
				adopt in order of pain: negentropy alone judges any repo — start there.
				add runseal when operations outgrow your shell history. add sidecar when
				one dev server becomes three cooperating processes. this site is the
				standing proof: its source is guarded by negentropy, its changes land
				through runseal wrappers, and its dev server rents its port from
				sidecar.
			</p>
			<Grid>
				<Card title="constitution">
					<p>
						the nine laws every repo in this workshop answers to, why each one
						exists, and the stability promise that prices change.
					</p>
					<Link to="/constitution">read the laws</Link>
				</Card>
				<Card title="vocabulary">
					<p>
						the living dictionary: every name declared across the three tools
						and this site, recounted on every site build — and written verdicts
						for the words that were fought over.
					</p>
					<Link to="/vocabulary">search the atoms</Link>
				</Card>
			</Grid>
		</article>
	);
}
