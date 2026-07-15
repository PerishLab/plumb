import { Card, Grid, Hero } from "@open-web/components";
import { Link } from "react-router";
import { title } from "../lib/title";

export function Home() {
	return (
		<article>
			<Hero
				title={title()}
				text="one small workshop, three sharp tools: a structural checker that keeps entropy down, an operator toolbelt that keeps flows explicit, and a process manager that keeps local runtimes named."
			/>
			<Grid>
				<Card title="negentropy">
					<p>
						a grammar-first structural checker enforcing an eight-law
						constitution: single-word names, shallow blocks and paths, denied
						comments, granted syntax, and a vocabulary that never freezes.
					</p>
					<Link to="/negentropy">read the checker</Link>
				</Card>
				<Card title="runseal">
					<p>
						the operator toolbelt: guard, land, and release flows plus forge
						tools, all run inside a small explicit profile with named wrappers
						and resource paths.
					</p>
					<Link to="/runseal">open the toolbelt</Link>
				</Card>
				<Card title="sidecar">
					<p>
						a manifest-driven process manager for multi-app dev: one plan from
						one manifest, stamped process identity, a namespace broker, and an
						inspect bridge.
					</p>
					<Link to="/sidecar">meet the manager</Link>
				</Card>
			</Grid>
			<Grid>
				<Card title="constitution">
					<p>
						the eight laws every repo in this workshop answers to, and the
						stability promise that prices change.
					</p>
					<Link to="/constitution">read the laws</Link>
				</Card>
				<Card title="vocabulary">
					<p>
						the living dictionary: every atom declared across the four repos,
						baked from negentropy itself and searchable here.
					</p>
					<Link to="/vocabulary">search the atoms</Link>
				</Card>
			</Grid>
		</article>
	);
}
