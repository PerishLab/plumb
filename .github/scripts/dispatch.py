import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from lib.backend import Backend
from lib.blob import Refusal, decode, encode, object_fields
from lib.declaration import Declaration
from lib.execution import Execution
from lib.inventory import Inventory
from lib.planner import plan
from lib.r2 import R2
from lib.source import Sources, resolve


JOBS = ("produce", "bind", "publish", "complete")


def emit(values):
    destination = os.environ.get("GITHUB_OUTPUT")
    if not destination:
        raise Refusal("workflow output channel is absent")
    with Path(destination).open("a", encoding="utf-8") as output:
        for name, value in values.items():
            value = str(value)
            if "\n" in value or "\r" in value:
                raise Refusal("workflow output must be one line")
            output.write(f"{name}={value}\n")


def prepare(value, root, control):
    object_fields(value, ("schema", "marker", "commit", "graph", "backend"))
    if value["schema"] != "plumb.ship-dispatch/v1":
        raise Refusal("unsupported Ship dispatch contract")
    commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], timeout=30).decode().strip()
    if commit != value["commit"] or value["marker"] != os.environ.get("PLUMB_RELEASE_MARKER"):
        raise Refusal("Ship dispatch differs from its checked-out marker")
    graph = Declaration(resolve(value["graph"], Sources({"product": root, "control": control})))
    backend = Backend(value["backend"])
    if set(backend.jobs) != set(JOBS):
        raise Refusal("declared backend differs from the central workflow jobs")
    for index, job in enumerate(JOBS):
        if set(backend.jobs[job]["needs"]) != set(JOBS[:index]):
            raise Refusal("declared barriers differ from the central workflow")
    backend.place(graph)
    return graph, backend


def main():
    parser = argparse.ArgumentParser(description="Cold-plan the centrally dispatched Ship graph")
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--control", required=True, type=Path)
    parser.add_argument("--job")
    parser.add_argument("--node")
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    inventory = Inventory(R2.environment())
    value = decode(inventory.download(os.environ.get("PLUMB_SHIP_DECLARATION", "")))
    graph, backend = prepare(value, args.root, args.control)
    seat = Path(tempfile.mkdtemp(prefix="ship-plan-", dir=os.environ.get("RUNNER_TEMP")))
    declaration = seat / "declaration.json"
    backend_file = seat / "backend.json"
    declaration.write_bytes(encode(value["graph"]))
    backend_file.write_bytes(encode(value["backend"]))
    if args.execute:
        from execute import perform
        args.backend = backend_file
        args.origin = os.environ["PLUMB_WORKFLOW_INVENTORY_URL"]
        args.state = seat / "state"
        args.executor = Path(__file__).with_name("ship.py")
        args.timeout = 3600
        result = perform(args, value["graph"], graph, inventory)
        print(encode(result).decode())
        return
    outputs = {"declaration": declaration, "backend": backend_file, "state": seat / "state"}
    if args.job is not None or args.node is not None:
        if backend.place(graph).get(args.node) != args.job:
            raise Refusal("execution node differs from its workflow placement")
        result = Execution(graph, inventory, os.environ["PLUMB_WORKFLOW_INVENTORY_URL"]).prepare(args.node)
        outputs["needed"] = str(result["state"] in ("run", "prepare")).lower()
    else:
        planned = plan(graph, inventory)
        for job in backend.jobs:
            matrix = backend.matrix(graph, planned, job)
            outputs[job] = encode(matrix).decode()
            outputs[job + "_needed"] = str(bool(matrix["include"])).lower()
    emit(outputs)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
