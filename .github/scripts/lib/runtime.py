import os
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path

from .blob import Refusal, decode, fingerprint
from .bundle import checksum, relative, unpack
from .material import download


def activate(request, seat, environment):
    references = [ref for ref in request.get("preparation", []) if ref["node"].startswith("runtime/")]
    if not references:
        return {}
    if len(references) != 1:
        raise Refusal("execution requires one declared runtime")
    reference = references[0]
    content = request["producers"][reference["node"]]["outputs"][reference["output"]]
    root = Path(tempfile.mkdtemp(prefix="rt-", dir=seat))
    archive = download(content, root / "runtime.zip")
    installation = root / "tools"
    installation.mkdir()
    with zipfile.ZipFile(archive) as bundle:
        value = decode(bundle.read("contract.json"))
        if fingerprint(value) != request["inputs"]["runtime"]["digest"]:
            raise Refusal("prepared runtime differs from the consumer tool world")
        for index, item in enumerate(value["archives"] + value["files"]):
            path = root / str(index)
            with bundle.open(str(index)) as source, path.open("xb") as target:
                shutil.copyfileobj(source, target)
            if checksum(path) != item["sha256"]:
                raise Refusal("prepared runtime archive checksum mismatch")
            if index < len(value["archives"]):
                unpack(path, item["prefix"], installation)
            else:
                target = installation / relative(item["path"])
                if target.exists() or target.is_symlink() or not target.resolve().is_relative_to(installation):
                    raise Refusal("runtime file projection overlaps or escapes its destination")
                target.parent.mkdir(parents=True, exist_ok=True)
                with path.open("rb") as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output)
                target.chmod(0o755 if item["executable"] else 0o644)
    changes = {name: str(root / relative(path)) for name, path in value["environment"].items()}
    changes["PATH"] = os.pathsep.join([str(root / relative(path)) for path in value["path"]]
                                     + [environment.get("PATH", "")])
    active = {**environment, **changes}
    for probe in value["probes"]:
        tool = installation / relative(probe["path"])
        actual = subprocess.check_output([str(tool), "--version"], env=active, timeout=30).decode().strip()
        if actual != probe["stdout"]:
            raise Refusal(f"prepared runtime tool differs: {probe['path']}; expected {probe['stdout']!r}, got {actual!r}")
    return changes
