import argparse
import base64
import gzip
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from lib.blob import Refusal, decode, encode, fingerprint, object_fields
from lib.bundle import relative
from lib.inventory import Inventory
from lib.readonly import Readonly


def configuration(body, destination):
    value = decode(body)
    object_fields(value, ("schema", "files"))
    if value["schema"] != "plumb.source-suite-configuration/v1" or not isinstance(value["files"], dict):
        raise Refusal("unsupported suite configuration media")
    if "guard-configuration.json" not in value["files"]:
        raise Refusal("suite configuration lacks its exact Guard manifest")
    destination.mkdir(exist_ok=False)
    for name, encoded in value["files"].items():
        path = destination / relative(name)
        if not isinstance(encoded, str):
            raise Refusal("configuration media requires base64 file bodies")
        try:
            body = base64.b64decode(encoded, validate=True)
        except ValueError as error:
            raise Refusal("configuration media has an invalid file body") from error
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("xb") as output:
            output.write(body)


def execute(request, destination):
    payload = request["payload"]
    object_fields(payload, ("schema", "source", "configuration", "platform", "target", "toolchain"))
    if payload["schema"] != "plumb.source-suite-work/v1":
        raise Refusal("unsupported source suite workload")
    if fingerprint(payload) != request["inputs"]["contract"]["digest"]:
        raise Refusal("source suite workload differs from its planned contract")
    materialized = request["materialized"]["source"]
    if materialized["key"] != request["inputs"]["source"]["digest"]:
        raise Refusal("source suite input differs from its planned snapshot")
    if payload["configuration"] != request["inputs"]["configuration"]["digest"]:
        raise Refusal("source suite configuration differs from its planned input")
    source = Path(materialized["root"]).resolve()
    seat = Path(tempfile.mkdtemp(prefix="suite-", dir=destination.parent)).resolve()
    rules = seat / "configuration"
    store = Inventory(Readonly(os.environ["PLUMB_WORKFLOW_INVENTORY_URL"]))
    configuration(store.download(payload["configuration"]), rules)
    allowed = ("PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC",
               "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "CARGO_HOME", "RUSTUP_HOME")
    active = {name: os.environ[name] for name in allowed if name in os.environ}
    target = seat / "target"
    active.update(RUSTUP_TOOLCHAIN=payload["toolchain"]["channel"], PLUMB_BUILD_SOURCE="1",
                  PLUMB_BUILD_COMMIT=payload["source"], PLUMB_GUARD_CONFIGURATION=str(rules),
                  CARGO_TARGET_DIR=str(target), CARGO_PROFILE_DEV_DEBUG="0", CARGO_INCREMENTAL="0")
    subprocess.run(["rustup", "toolchain", "install", active["RUSTUP_TOOLCHAIN"],
                    "--profile", "minimal"], env=active, check=True, timeout=600)
    for tool in ("rustc", "cargo"):
        actual = subprocess.check_output([tool, "--version"], env=active, timeout=30).decode().strip()
        if actual != payload["toolchain"][tool]:
            raise Refusal("source suite tool differs from its declared world: " + tool)
    host = subprocess.check_output(["rustc", "-vV"], env=active, timeout=30).decode().splitlines()
    if "host: " + payload["target"] not in host:
        raise Refusal("source suite requires its declared native platform")
    metadata = decode(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                              cwd=source, env=active, timeout=60))
    packages = [package for package in metadata["packages"] if package["name"] == "plumb-cli"]
    if len(packages) != 1:
        raise Refusal("source suite requires exactly one Plumb CLI package")
    version = "v" + packages[0]["version"]
    subprocess.run(["cargo", "build", "--locked", "--bin", "plumb"], cwd=source,
                   env=active, check=True, timeout=3600)
    binary = target / "debug" / ("plumb.exe" if payload["platform"] == "windows" else "plumb")
    identity = subprocess.check_output([str(binary), "--version"], env=active, timeout=30).decode().strip()
    if identity != "plumb " + version:
        raise Refusal("built source binary differs from its native package identity")
    subprocess.run(["cargo", "test", "--locked", "--workspace"], cwd=source,
                   env=active, check=True, timeout=3600)
    python = dict(active, PYTHONPATH=str(source / ".github/scripts"))
    subprocess.run([sys.executable, "-B", "-m", "unittest", "discover", "-s", ".github/scripts/tests",
                    "-p", "test_*.py"], cwd=source, env=python, check=True, timeout=600)
    archive = seat / "binary.gz"
    with binary.open("rb") as content, archive.open("xb") as output:
        with gzip.GzipFile(filename="", mode="wb", mtime=0, fileobj=output) as compressed:
            shutil.copyfileobj(content, compressed)
    receipt = seat / "receipt.json"
    receipt.write_bytes(encode({"schema": "plumb.source-suite-result/v1", "contract": payload,
                                "version": version, "snapshot": materialized["key"]}))
    destination.write_bytes(encode({"key": request["key"], "outputs": {"binary": str(archive)},
                                    "evidence": {"receipt": str(receipt)}}))


def main():
    parser = argparse.ArgumentParser(description="Build and test one unprivileged native source suite")
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
