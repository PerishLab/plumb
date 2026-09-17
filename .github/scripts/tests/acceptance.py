import argparse
import gzip
import importlib.util
import json
import os
import shutil
import sys
from pathlib import Path
from unittest.mock import patch


def main():
    parser = argparse.ArgumentParser(description="Exercise worker installation with the actual Cargo-built CLI")
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--binding", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--seat", required=True, type=Path)
    args = parser.parse_args()
    scripts = Path(__file__).resolve().parents[1]
    sys.path.insert(0, str(scripts))
    target = args.root / ".github/scripts/ship.py"
    target.parent.mkdir(parents=True)
    shutil.copyfile(scripts / "ship.py", target)
    spec = importlib.util.spec_from_file_location("worker", target)
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)
    binding = json.loads(args.binding)
    request = {"payload": {"controller": binding},
               "preparation": [{"node": "prepare/linux", "output": "content"}],
               "producers": {"prepare/linux": {"outputs": {"content": {"digest": "a" * 64}}}}}

    def archive(reference, destination):
        destination.write_bytes(gzip.compress(args.binary.read_bytes(), compresslevel=1, mtime=0))
        return destination

    environment = dict(os.environ, PLUMB_RULES_SOURCE=args.source)
    with patch.object(worker, "download", archive):
        binary = worker.controller(request, args.seat, environment)
    assert Path(binary).read_bytes() == args.binary.read_bytes()
    installed = json.loads((args.seat / "home/configurations/latest.json").read_bytes())
    assert installed["marker"] == binding["marker"]
    assert installed["generation"] == binding["generation"]
    assert not (args.root / ".git/hooks/pre-commit").exists()


if __name__ == "__main__":
    main()
