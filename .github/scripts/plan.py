import argparse
import json
import sys
from pathlib import Path

from lib.blob import Refusal, decode
from lib.backend import Backend
from lib.declaration import Declaration
from lib.inventory import Inventory
from lib.planner import plan
from lib.r2 import R2
from lib.source import Sources, resolve


def main():
    parser = argparse.ArgumentParser(description="Plan opaque blob actions without business tools")
    parser.add_argument("--declaration", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--control", type=Path, help="Exact checked-out control implementation source")
    parser.add_argument("--backend", type=Path)
    args = parser.parse_args()
    sources = Sources({"product": args.root, "control": args.control})
    graph = Declaration(resolve(decode(args.declaration.read_bytes()), sources))
    backend = None if args.backend is None else Backend(decode(args.backend.read_bytes()))
    if backend:
        backend.place(graph)
    planned = plan(graph, Inventory(R2.environment()))
    if backend:
        planned["backend"] = backend.project(graph, planned)
    print(json.dumps(planned, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
