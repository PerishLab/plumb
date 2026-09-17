import argparse
import sys
from pathlib import Path

from lib.blob import Refusal, Unknown
from lib.inventory import Inventory
from lib.r2 import R2


def main():
    parser = argparse.ArgumentParser(description="Publish immutable bytes into the workflow blob store")
    parser.add_argument("source", type=Path)
    args = parser.parse_args()
    print(Inventory(R2.environment()).upload(args.source.read_bytes()))


if __name__ == "__main__":
    try:
        main()
    except (Refusal, Unknown, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
