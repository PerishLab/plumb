import { Badge, Card, Code, Grid, Hero } from "@open-web/components";

const sample = `$ sidecar plan --config examples/minimal.toml
project: example-sidecar-project (namespace: default)
app: desktop -> pnpm tauri dev
targets: 2
- api [mode=dev] -> cargo run -p example-api -- --sidecar-stamp=v=1;a=api;n=default;m=dev;s=tool%3Asidecar
    inspect_socket: unix:///tmp/sidecar-example-api.sock
- desktop [mode=dev] -> pnpm tauri dev --sidecar-stamp=v=1;a=desktop;n=default;m=dev;s=tool%3Asidecar
inspect endpoints: 1
- api-health http http://127.0.0.1:3901/health`;

export function Sidecar() {
	return (
		<article>
			<Hero
				title="sidecar"
				text="a lightweight, manifest-driven process instance manager for projects that run a small set of cooperating local processes: shallow isolation without space isolation."
			>
				<Badge>manifest</Badge> <Badge>stamp</Badge> <Badge>broker</Badge>{" "}
				<Badge>inspect</Badge>
			</Hero>
			<Grid>
				<Card title="manifest">
					<p>
						sidecar.toml is the lifecycle contract, not a launch snippet:
						command, cwd, args, env, readiness, inspect socket, stop behavior,
						and reset boundary are declared up front.
					</p>
				</Card>
				<Card title="stamp">
					<p>
						every spawned target receives one packed --sidecar-stamp arg
						carrying identity and the runtime endpoint. it is the only launch
						metadata contract — no env fallback.
					</p>
				</Card>
				<Card title="broker">
					<p>
						each project and namespace gets one loopback tcp broker, discovered
						from argv identity plus live listener probing and confirmed with a
						hello handshake.
					</p>
				</Card>
				<Card title="inspect">
					<p>
						one event frame over a unix socket talks to a running target. the
						project owns event names and payloads; sidecar owns the transport
						envelope and timeout.
					</p>
				</Card>
			</Grid>
			<Code>{sample}</Code>
		</article>
	);
}
