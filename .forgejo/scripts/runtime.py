import argparse
import shutil
import sys
import tempfile
import zipfile
from pathlib import Path
from urllib.parse import urlsplit

from lib.blob import Refusal, Unknown, decode, encode, fingerprint, object_fields, sha
from lib.bundle import checksum, relative
from lib.transport import exchange


def contract(value):
    object_fields(value, ("schema", "archives", "files", "probes", "environment", "path"))
    if value["schema"] != "plumb.runtime-bundle/v1" or not value["archives"] or not value["probes"]:
        raise Refusal("unsupported runtime bundle")
    if any(not isinstance(value[key], list) for key in ("archives", "files", "probes", "path")):
        raise Refusal("runtime requires explicit archive, probe and path lists")
    if not isinstance(value["environment"], dict) or not value["path"]:
        raise Refusal("runtime requires an environment mapping and executable path")
    for index, archive in enumerate(value["archives"] + value["files"]):
        if index < len(value["archives"]):
            object_fields(archive, ("url", "sha256", "prefix"))
            relative(archive["prefix"])
        else:
            object_fields(archive, ("url", "sha256", "path", "executable"))
            relative(archive["path"])
            if not isinstance(archive["executable"], bool):
                raise Refusal("runtime file requires an executable flag")
        sha(archive["sha256"])
        url = urlsplit(archive["url"])
        if url.scheme != "https" or not url.hostname or url.username or url.password or url.query or url.fragment:
            raise Refusal("runtime archive requires an uncredentialed HTTPS URL")
    for probe in value["probes"]:
        object_fields(probe, ("path", "stdout"))
        relative(probe["path"])
        if not isinstance(probe["stdout"], str) or not probe["stdout"]:
            raise Refusal("runtime probe requires exact stdout")
    for name, path in value["environment"].items():
        if not name or not all(character.isupper() or character == "_" for character in name):
            raise Refusal("runtime environment requires explicit variable names")
        relative(path)
    for path in value["path"]:
        relative(path)
    return value


def fetch(archive, path):
    url = urlsplit(archive["url"])
    try:
        status, _, body = exchange(archive["url"], "GET", {"Host": url.netloc}, b"")
    except OSError as error:
        raise Unknown("runtime source unavailable: " + type(error).__name__) from error
    if status != 200:
        raise Unknown(f"runtime source unavailable: HTTP {status}")
    path.write_bytes(body)
    if checksum(path) != archive["sha256"]:
        raise Refusal("runtime source checksum mismatch")


def execute(request, destination):
    value = contract(request["payload"])
    if request["inputs"]["contract"]["digest"] != fingerprint(value):
        raise Refusal("runtime bundle differs from its declared input")
    seat = Path(tempfile.mkdtemp(prefix="runtime-", dir=destination.parent))
    output = seat / "runtime.zip"
    with zipfile.ZipFile(output, "x", compression=zipfile.ZIP_STORED) as bundle:
        bundle.writestr(zipfile.ZipInfo("contract.json"), encode(value))
        for index, archive in enumerate(value["archives"] + value["files"]):
            path = seat / str(index)
            fetch(archive, path)
            with path.open("rb") as source, bundle.open(zipfile.ZipInfo(str(index)), "w", force_zip64=True) as target:
                shutil.copyfileobj(source, target)
    proof = seat / "receipt.json"
    proof.write_bytes(encode({"schema": "plumb.runtime-proof/v1", "contract": fingerprint(value)}))
    destination.write_bytes(encode({"key": request["key"], "outputs": {"content": str(output)},
                                    "evidence": {"receipt": str(proof)}}))


def main():
    parser = argparse.ArgumentParser(description="Prepare a locked portable runtime through the blob contract")
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--result", required=True, type=Path)
    args = parser.parse_args()
    execute(decode(args.request.read_bytes()), args.result)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
