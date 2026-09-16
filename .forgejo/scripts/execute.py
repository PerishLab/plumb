import argparse
import json
import subprocess
import sys
from pathlib import Path

from lib.backend import Backend
from lib.blob import Refusal, decode, encode
from lib.declaration import Declaration
from lib.execution import Execution
from lib.inventory import Inventory
from lib.inputs import Inputs
from lib.r2 import R2
from lib.runner import Runner
from lib.source import Sources, resolve


class Executor:
    def __init__(self, script, root, timeout, inputs=None):
        if timeout <= 0:
            raise Refusal("executor timeout must be positive")
        self.script = Path(script).resolve()
        self.root = Path(root).resolve()
        self.timeout = timeout
        self.inputs = inputs or Inputs({})

    def __call__(self, request, seat):
        source = seat / "request.json"
        result = seat / "candidate.json"
        result.unlink(missing_ok=True)
        materialized = self.inputs.materialize(request, seat)
        source.write_bytes(encode({**request, "materialized": materialized}))
        command = [sys.executable, "-B", str(self.script), "--request", str(source),
                   "--result", str(result)]
        try:
            status = subprocess.run(command, cwd=self.root, timeout=self.timeout)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise Refusal("executor interrupted; its destination requires reconciliation") from error
        if status.returncode:
            raise Refusal("executor failed; its destination requires reconciliation")
        return decode(result.read_bytes())


def main():
    parser = argparse.ArgumentParser(description="Execute one declared blob node with local record recovery")
    parser.add_argument("--declaration", required=True, type=Path)
    parser.add_argument("--backend", required=True, type=Path)
    parser.add_argument("--job", required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--control", type=Path, help="Exact checked-out control implementation source")
    parser.add_argument("--origin", required=True)
    parser.add_argument("--state", required=True, type=Path)
    parser.add_argument("--executor", required=True, type=Path,
                        help="Central business bridge accepting --request and --result JSON paths")
    parser.add_argument("--timeout", type=int, default=3600)
    args = parser.parse_args()
    declaration = decode(args.declaration.read_bytes())
    source = Sources({"product": args.root, "control": args.control})
    graph = Declaration(resolve(declaration, source))
    print(json.dumps(perform(args, declaration, graph, Inventory(R2.environment())), sort_keys=True))


def perform(args, declaration, graph, inventory):
    source = Sources({"product": args.root, "control": args.control})
    backend = Backend(decode(args.backend.read_bytes()))
    placement = backend.place(graph)
    if placement.get(args.node) != args.job:
        raise Refusal("node is not assigned to this workflow job")
    execution = Execution(graph, inventory, args.origin)
    def executor(identity):
        if placement.get(identity) != args.job:
            raise Refusal("preparation is outside the executing workflow job")
        script = args.executor
        if graph.nodes[identity]["execution"]["entry"].startswith("controller."):
            script = Path(__file__).with_name("controller.py")
        return Executor(script, args.root, args.timeout, Inputs.select(declaration, identity, source))

    return Runner(execution, args.state).run(args.node, executor(args.node), executor)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
