import { Card, Grid, Hero, Rail } from "@open-web/components";
import { Link } from "react-router";
import { title } from "../lib/title";

export function Home() {
	return (
		<article>
			<Hero
				mark="/favicon.svg"
				title={title()}
				text="a workshop for code that AI agents maintain. the problem is familiar: an agent rewrites the file, the comment above it quietly becomes a lie, and nobody — human or agent — rereads it. so the rules here live where a checker can enforce them: in structure and named vocabulary, not in prose."
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
						keep flows explicit: declared resources, built-in forge tools, and
						repo-authored wrappers for flows like guard, land, and release — all
						inside one small explicit profile.
					</p>
					<Link to="/runseal">open the toolbelt</Link>
				</Card>
				<Card title="sidecar">
					<p>
						keep local runtimes named: one manifest per project — stamped
						process identity, a chosen free port handed to each target, one
						local broker per namespace, and a live inspect bridge.
					</p>
					<Link to="/sidecar">meet the manager</Link>
				</Card>
			</Grid>
			<h2>adopt in order of pain</h2>
			<Rail
				stops={[
					{
						mark: "/marks/negentropy.svg",
						name: "negentropy",
						text: "any repo it can read — start here.",
					},
					{
						mark: "/marks/runseal.svg",
						name: "runseal",
						text: "when operations outgrow your shell history.",
					},
					{
						mark: "/marks/sidecar.svg",
						name: "sidecar",
						text: "when one dev server becomes three cooperating processes.",
					},
				]}
			/>
			<p>
				this site practices what it preaches: its source is checked by
				negentropy, its changes land through runseal wrappers, and its dev
				server rents its port from sidecar.
			</p>
			<Grid>
				<Card title="constitution">
					<p>
						the nine laws every repo in this workshop answers to, why each one
						exists, and the versioning promise that makes a rule change cost a
						release.
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
