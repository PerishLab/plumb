import argparse
import gzip
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from lib.blob import Refusal, decode, encode, fingerprint, object_fields
from lib.process import environment


def execute(request, destination):
    contract = request.get("payload")
    object_fields(contract, ("schema", "version", "channel", "commit", "target", "rustc", "cargo", "environment", "format"))
    if contract["schema"] != "plumb.controller-build/v1" or contract["format"] != "gzip-6":
        raise Refusal("unsupported controller production contract")
    if fingerprint(contract) != request["inputs"]["contract"]["digest"]:
        raise Refusal("controller production differs from its declared input")
    materialized = request["materialized"]["source"]
    if materialized["key"] != request["inputs"]["source"]["digest"]:
        raise Refusal("controller source differs from its planned snapshot")
    source = Path(materialized["root"]).resolve()
    for tool in ("rustc", "cargo"):
        actual = subprocess.check_output([tool, "--version"], cwd=source, timeout=30).decode().strip()
        if actual != contract[tool]:
            raise Refusal("controller tool differs from its declared world: " + tool)
    seat = Path(tempfile.mkdtemp(prefix="controller-", dir=destination.parent)).resolve()
    target = seat / "target"
    managed = dict(PLUMB_BUILD_VERSION=contract["version"],
                       PLUMB_BUILD_CHANNEL=contract["channel"],
                       PLUMB_BUILD_COMMIT=contract["commit"], PLUMB_BUILD_SOURCE="1",
                       CARGO_PROFILE_DEV_DEBUG="0", CARGO_INCREMENTAL="0",
                       CARGO_TARGET_DIR=str(target),
                       RUSTFLAGS=f"--remap-path-prefix={source}=/plumb/source --remap-path-prefix={target}=/plumb/target")
    command = ["cargo", "build", "--locked", "--bin", "plumb", "--target", contract["target"]]
    subprocess.run(command, cwd=source, env=environment(contract["environment"], managed), timeout=3600, check=True)
    executable = "plumb.exe" if "windows" in contract["target"] else "plumb"
    content = target / contract["target"] / "debug" / executable
    if not content.is_file():
        raise Refusal("controller build produced no executable")
    archive = seat / "controller.gz"
    with content.open("rb") as source, archive.open("xb") as output:
        with gzip.GzipFile(filename="", mode="wb", compresslevel=6, mtime=0, fileobj=output) as compressed:
            shutil.copyfileobj(source, compressed)
    proof = seat / "receipt.json"
    proof.write_bytes(encode({"schema": "plumb.controller-proof/v1", "contract": contract,
                             "source": materialized["key"]}))
    destination.write_bytes(encode({"key": request["key"], "outputs": {"content": str(archive)},
                                    "evidence": {"receipt": str(proof)}}))


def main():
    parser = argparse.ArgumentParser(description="Build one declared controller workload without Plumb bootstrap")
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--result", required=True, type=Path)
    args = parser.parse_args()
    execute(decode(args.request.read_bytes()), args.result)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
