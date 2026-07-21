import {
	Badge,
	Banner,
	Card,
	Code,
	Forge,
	Grid,
} from "@perish/react-components";
import releases from "../data/releases.json";

const install = `import { assert, family, kind, run } from "jsr:@perish/shield";`;

const declare = `// land.ts - one domain declares its failure vocabulary once
export const fault = family("land", {
	dirty: kind<{ branch: string }>(),
	missing: kind<{ path: string }>(),
	ahead: kind<{ base: string; by: number }>(),
});`;

const raise = `// throw at the disease, deep in the flow
assert(clean, () => fault.dirty({ branch }));`;

const consume = `// dispose at the boundary - every kind, or the compiler stops you
await run(land).catch(fault.consume({
	dirty: (f) => io.fail(\`dirty on \${f.meta.branch}\`),
	missing: (f) => io.fail(\`no path \${f.meta.path}\`),
	ahead: (f) => io.fail(\`behind by \${f.meta.by}\`),
}));`;

export function Shield() {
	return (
		<article>
			<Banner mark="/marks/shield.svg" title="shield" line="keep faults named.">
				<Badge>stable {releases.shield.version}</Badge>
				<Forge repo="PerishFire/shield" />
			</Banner>
			<p>
				error handling rots differently: a thrown value loses its shape at the
				first catch, so a boundary sees an unknown and every handler re-guesses
				what went wrong. shield keeps native throw but gives each failure a name
				— a domain declares its fault vocabulary once, and that one declaration
				types both the throw site and every handler.
			</p>
			<p>
				<Badge>native throw</Badge> <Badge>no Result monad</Badge>{" "}
				<Badge>zero deps</Badge> <Badge>below harness</Badge>
			</p>
			<h2 id="quickstart">quickstart</h2>
			<p>
				shield is a jsr package, runtime-pure with no dependencies. a business
				brings its own schema (zod, or the zero-dep kind phantom shown here);
				shield consumes the inferred shape and never validates — validation, if
				you want it, is the business's own call.
			</p>
			<Code name="import" copy>
				{install}
			</Code>
			<Code name="land.ts">{declare}</Code>
			<p>
				the spec is the single source of truth: kinds and their payload shapes
				flow into both the runtime factories and the type of every handler.
			</p>
			<Code>{raise}</Code>
			<Code>{consume}</Code>
			<Grid>
				<Card title="one source of truth">
					<p>
						family(name, spec) declares a domain's failure kinds once; the
						factories and every handler's meta type derive from it. no compound
						name is minted — the surface is a value on the domain module.
					</p>
				</Card>
				<Card title="native throw, no monad">
					<p>
						a fault propagates by being thrown, never wrapped in a Result. run
						absorbs every non-fault into a foreign kind, so a handler always
						faces a fault, and a bug still crashes loudly unless routed on
						purpose.
					</p>
				</Card>
				<Card title="the boundary consumes">
					<p>
						consume is exhaustive over a family's kinds — every kind handled, or
						the compiler stops you. add a kind and every boundary must
						reconsider. compose boundaries by chaining catch.
					</p>
				</Card>
				<Card title="below harness">
					<p>
						shield is a language substrate under harness: runtime-pure, no
						console, no runtime globals. adapters at the harness seam mint
						faults from native errors, so shield never touches the platform.
					</p>
				</Card>
			</Grid>
		</article>
	);
}
