import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from dispatch import prepare
from execute import perform
from lib.blob import Refusal, decode, encode
from lib.inventory import Inventory
from lib.planner import plan
from lib.r2 import R2


def checkout(root, seat):
    directory = Path(tempfile.mkdtemp(prefix="node-", dir=seat)) / "source"
    origin = subprocess.check_output(["git", "-C", str(root), "remote", "get-url", "origin"], timeout=30).decode().strip()
    subprocess.run(["git", "clone", "--local", "--no-hardlinks", str(root), str(directory)], check=True, timeout=120)
    subprocess.run(["git", "-C", str(directory), "remote", "set-url", "origin", origin], check=True, timeout=30)
    return directory


def main():
    parser = argparse.ArgumentParser(description="Recover declared Ship nodes on one compatible local host")
    parser.add_argument("--declaration", required=True, type=Path)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--control", required=True, type=Path)
    parser.add_argument("--runners", required=True)
    args = parser.parse_args()
    value = decode(args.declaration.read_bytes())
    graph, backend = prepare(value, args.root, args.control)
    runners = set(decode(args.runners.encode()))
    seat = Path(tempfile.mkdtemp(prefix="local-ship-", dir=os.environ.get("RUNNER_TEMP")))
    args.backend = seat / "backend.json"
    args.backend.write_bytes(encode(value["backend"]))
    args.state = seat / "state"
    args.origin = os.environ["PLUMB_WORKFLOW_INVENTORY_URL"]
    args.executor = Path(__file__).with_name("ship.py")
    args.timeout = 3600
    inventory = Inventory(R2.environment())
    source_root = args.root
    executed = set()
    while True:
        held = plan(graph, inventory)
        if held["complete"]:
            print(encode({"complete": True, "executed": sorted(executed)}).decode())
            return
        progress = False
        for job in backend.jobs:
            for row in backend.matrix(graph, held, job)["include"]:
                identity = row["node"]
                if row["runner"] not in runners or identity in executed:
                    continue
                current = held["nodes"][identity]
                preparation = {ref["node"] for ref in graph.nodes[identity].get("prepare", [])}
                if current["key"] is None or not set(current["waiting"]) <= preparation:
                    continue
                args.job, args.node = job, identity
                args.root = checkout(source_root, seat)
                perform(args, value["graph"], graph, inventory)
                executed.add(identity)
                progress = True
        if not progress:
            pending = [key for key, node in held["nodes"].items() if node["state"] in ("run", "pending")]
            raise Refusal("local Ship remains incomplete; pending nodes: " + ", ".join(pending))


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
