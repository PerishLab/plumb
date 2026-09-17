import argparse
import sys
from pathlib import Path

from lib.blob import Refusal, decode, encode, sha
from lib.source import Sources


def main():
    parser = argparse.ArgumentParser(description="Hash and materialize the same declared input bytes")
    parser.add_argument("--recipe", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--control", type=Path)
    parser.add_argument("--expect", help="Require the key previously accepted by plan")
    parser.add_argument("--destination", type=Path, help="Materialize into a new path and verify readback")
    args = parser.parse_args()
    snapshot = Sources({"product": args.root, "control": args.control}).snapshot(decode(args.recipe.read_bytes()))
    if args.expect is not None and sha(args.expect) != snapshot.key:
        raise Refusal("execution input differs from the planned input key")
    if args.destination is not None:
        snapshot.materialize(args.destination)
    print(encode({"key": snapshot.key, "manifest": snapshot.manifest}).decode())


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
