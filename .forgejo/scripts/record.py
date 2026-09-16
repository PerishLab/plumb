import argparse
import json
import sys
from pathlib import Path

from lib.blob import Refusal, decode
from lib.completion import complete
from lib.declaration import Declaration
from lib.inventory import Inventory
from lib.r2 import R2
from lib.source import Source, resolve


def main():
    parser = argparse.ArgumentParser(description="Record readable blobs after executor success")
    parser.add_argument("--declaration", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--node", required=True)
    parser.add_argument("--result", required=True, type=Path,
                        help="JSON result with the executed key and output-name to local-file map")
    args = parser.parse_args()
    graph = Declaration(resolve(decode(args.declaration.read_bytes()), Source(args.root)))
    result = decode(args.result.read_bytes())
    print(json.dumps(complete(graph, args.node, result, Inventory(R2.environment())),
                     sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
