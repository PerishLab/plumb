import hashlib
import os
import shutil
import tarfile
from pathlib import Path, PurePosixPath

from .blob import Refusal


def relative(value):
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        raise Refusal("runtime path is not portable")
    if PurePosixPath(value).is_absolute() or any(part in ("", ".", "..") for part in value.split("/")):
        raise Refusal("runtime path escapes its destination")
    return value


def checksum(path):
    result = hashlib.sha256()
    with Path(path).open("rb") as source:
        for part in iter(lambda: source.read(1024 * 1024), b""):
            result.update(part)
    return result.hexdigest()


def unpack(archive, prefix, destination):
    prefix = relative(prefix) + "/"
    destination = Path(destination).resolve()
    links = []
    count = 0
    with tarfile.open(archive, "r:*") as package:
        for member in package:
            if not member.name.startswith(prefix):
                continue
            name = member.name[len(prefix):].rstrip("/")
            if not name or name == "manifest.in":
                continue
            path = destination / relative(name)
            if not path.resolve().is_relative_to(destination):
                raise Refusal("runtime archive escapes its destination")
            if member.isdir():
                path.mkdir(parents=True, exist_ok=True)
                continue
            if path.exists() or path.is_symlink():
                raise Refusal("runtime archives overlap")
            path.parent.mkdir(parents=True, exist_ok=True)
            if member.issym():
                target = (path.parent / relative(member.linkname)).resolve()
                if not target.is_relative_to(destination):
                    raise Refusal("runtime link escapes its destination")
                links.append((path, target))
            elif member.isfile():
                with package.extractfile(member) as source, path.open("xb") as output:
                    shutil.copyfileobj(source, output)
                path.chmod(0o755 if member.mode & 0o111 else 0o644)
            else:
                raise Refusal("unsupported runtime archive entry")
            count += 1
    for path, target in links:
        if not target.is_file() or target.is_symlink():
            raise Refusal("runtime link requires an installed regular file")
        if os.name == "nt":
            shutil.copyfile(target, path)
        else:
            path.symlink_to(os.path.relpath(target, path.parent))
    if not count:
        raise Refusal("runtime archive projection is empty")
