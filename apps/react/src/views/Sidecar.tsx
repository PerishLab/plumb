import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://sidecar.perish.uk/manage.sh | sh`;

const manifest = `[project]
name = "open-web"
namespace = "open-web"

[app]
name = "react"
command = "pnpm"
args = ["--filter", "@open-web/react", "dev", "--"]
mode = "dev"
port = 0
health_url = "http://127.0.0.1:{port}"`;

const session = `$ sidecar start
broker runtime pid=1355780 endpoint=tcp://127.0.0.1:40185
started react pid=1355785

$ sidecar status
namespace: open-web
runtime: running (pid 1355780) tcp://127.0.0.1:40185
- react: running (pid 1355785) http://127.0.0.1:34953`;

export function Sidecar() {
	return (
		<article>
			<Banner
				mark="/marks/sidecar.svg"
				title="sidecar"
				line="keep local runtimes named."
			>
				<Badge>stable {releases.sidecar}</Badge>
				<Forge repo="PerishCode/sidecar" />
			</Banner>
			<p>
				local processes share one host — PATH, credentials, localhost, ports,
				logs. once a project runs more than one communicating process, that
				shared space needs machine-readable identity, or dev servers, workers,
				and agent sessions cross wires: wrong endpoints, stale pids, ambiguous
				logs, unsafe cleanup. sidecar is shallow isolation without space
				isolation — not a container runtime, not a cluster, not pm2.
			</p>
			<p>
				<Badge>manifest</Badge> <Badge>stamp</Badge> <Badge>broker</Badge>{" "}
				<Badge>inspect</Badge>
			</p>
			<h2>quickstart</h2>
			<Code copy>{start}</Code>
			<p>
				manage.sh fetches the released binary from R2 into ~/.local/bin — linux,
				macos, and windows. the lifecycle contract is one manifest at the repo
				root. this one is not an example: it is this site's own, and the page
				you are reading was served through a port leased exactly this way.
			</p>
			<Code>{manifest}</Code>
			<Code>{session}</Code>
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
				<Card title="port lease">
					<p>
						port = 0 leases a free loopback port at every start, injected as
						SIDECAR_PORT and substituted into health_url — no fixed port, no
						loopback squatting, nothing to collide with.
					</p>
				</Card>
			</Grid>
		</article>
	);
}
