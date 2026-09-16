import argparse
import gzip
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from lib.blob import Refusal, decode, encode
from lib.material import download


def controller(request, seat, environment):
    preparation = request.get("preparation", [])
    if len(preparation) != 1:
        raise Refusal("Ship requires one declared controller preparation")
    reference = preparation[0]
    content = request["producers"][reference["node"]]["outputs"][reference["output"]]
    archive = download(content, seat / "controller.gz")
    tool = seat / ("plumb.exe" if os.name == "nt" else "plumb")
    with gzip.open(archive, "rb") as source, tool.open("xb") as output:
        shutil.copyfileobj(source, output)
    tool.chmod(0o755)
    configuration = os.environ.get("PLUMB_BUILD_CONFIGURATION")
    if not configuration:
        raise Refusal("Ship controller configuration is absent")
    environment["PLUMB_HOME"] = str(seat / "home")
    subprocess.run([str(tool), "configuration", "install", "--version", configuration],
                   env=environment, timeout=300, check=True)
    return str(tool)


def execute(source, destination):
    request = decode(source.read_bytes())
    if request.get("schema") != "plumb.blob-execution/v1":
        raise Refusal("Ship bridge requires a blob execution context")
    seat = Path(tempfile.mkdtemp(prefix="ship-", dir=destination.parent)).resolve()
    result = seat / "result.json"
    environment = dict(os.environ,
                       PLUMB_RELEASE_ARTIFACTS=str(seat / "artifacts"),
                       PLUMB_RELEASE_OUTPUT=str(seat / "capsule"),
                       PLUMB_RELEASE_CAPSULE=str(seat / "capsule/capsule.json"),
                       PLUMB_RELEASE_PROMOTION=str(seat / "promotion.json"))
    tool = controller(request, seat, environment)
    status = subprocess.run([tool, "ship", "execute", "--request-file", str(source.resolve()),
                             "--output", str(result)], env=environment, timeout=3600)
    if status.returncode:
        raise Refusal("Ship business execution failed; reconcile its destination before retry")
    held = decode(result.read_bytes())
    if held.get("schema") != "plumb.ship-result/v2" or held.get("action") != request.get("node"):
        raise Refusal("Ship result differs from its execution context")
    projection = held.get("projection")
    if not isinstance(projection, dict) or not isinstance(projection.get("workload"), str):
        raise Refusal("Ship result has no workload file")
    content = Path(projection["workload"]).resolve()
    if not content.is_file():
        raise Refusal("Ship result workload is absent")
    receipt = seat / "receipt.json"
    if not isinstance(held.get("evidence"), dict):
        raise Refusal("Ship result has no business evidence")
    receipt.write_bytes(encode(held["evidence"]))
    destination.write_bytes(encode({"key": request["key"], "outputs": {"content": str(content)},
                                    "evidence": {"receipt": str(receipt)}}))


def main():
    parser = argparse.ArgumentParser(description="Transport a planned request to the Plumb business executor")
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--result", required=True, type=Path)
    args = parser.parse_args()
    execute(args.request, args.result)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
