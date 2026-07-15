import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://sidecar.perish.uk/manage.sh | sh`;

const manifest = `[project]
name = "smoke"
namespace = "smoke"

[app]
name = "echo"
command = "sh"
args = ["-c", "exec python3 -m http.server $SIDECAR_PORT --bind 127.0.0.1"]
mode = "dev"
port = 0
health_url = "http://127.0.0.1:{port}"`;

const session = `$ sidecar start
broker runtime pid=1258583 endpoint=tcp://127.0.0.1:42071
started echo pid=1258588

$ sidecar status
namespace: smoke
runtime: running (pid 1258583) tcp://127.0.0.1:42071
- echo: running (pid 1258588) http://127.0.0.1:36761`;

const living = `[app]
name = "react"
command = "pnpm"
args = ["--filter", "@open-web/react", "dev", "--"]
port = 0
health_url = "http://127.0.0.1:{port}"

$ sidecar status
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
				root. this one runs anywhere python runs — the port is leased at start
				and handed to the target as SIDECAR_PORT:
			</p>
			<Code>{manifest}</Code>
			<Code>{session}</Code>
			<p>
				and the same contract in production: the page you are reading was served
				through a port leased exactly this way, from this site's own manifest.
			</p>
			<Code>{living}</Code>
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
