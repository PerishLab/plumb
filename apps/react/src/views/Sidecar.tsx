import { Badge, Banner, Card, Code, Forge, Grid } from "@open-web/components";
import releases from "../data/releases.json";

const start = `curl -fsSL https://sidecar.perish.uk/manage.sh | sh`;

const manifest = `[project]
name = "smoke"
namespace = "smoke"

[app]
name = "echo"
command = "sh"
args = ["-c", "python3 -m http.server $SIDECAR_PORT --bind 127.0.0.1"]
mode = "dev"
port = 0
health_url = "http://127.0.0.1:{port}"`;

const session = `$ sidecar start
broker runtime pid=1467115 endpoint=tcp://127.0.0.1:42831
started echo pid=1467120

$ sidecar status
namespace: smoke
runtime: running (pid 1467115) tcp://127.0.0.1:42831
- echo: running (pid 1467120) http://127.0.0.1:44247`;

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
			<h2 id="quickstart">quickstart</h2>
			<Code name="install" copy>
				{start}
			</Code>
			<p>
				manage.sh installs linux x86_64 and macos (intel and apple silicon) and
				links sidecar into ~/.local/bin — put that on PATH; windows has
				manage.ps1. the lifecycle contract is one manifest at the repo root.
				this one needs only sh and python: sidecar picks a free loopback port at
				start and hands it to the target as SIDECAR_PORT:
			</p>
			<Code name="sidecar.toml">{manifest}</Code>
			<Code>{session}</Code>
			<p>
				and the same contract runs this site's own dev server: its manifest
				below is the real one — the port is picked before pnpm dev starts.
			</p>
			<Code name="sidecar.toml">{living}</Code>
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
						carrying identity and the runtime endpoint. identity is stamp-only —
						a chosen port still arrives separately, as env.
					</p>
				</Card>
				<Card title="broker">
					<p>
						each project and namespace gets one local tcp broker: sidecar finds
						it by its process stamp, probes the listener, and trusts it only
						after the reply.
					</p>
				</Card>
				<Card title="port lease">
					<p>
						port = 0 asks the kernel for a free loopback port at every start,
						passes the number as SIDECAR_PORT and into health_url — the target
						owns the bind, and nothing squats a fixed port.
					</p>
				</Card>
			</Grid>
		</article>
	);
}
