import os
import stat
from pathlib import Path, PurePosixPath

from .blob import Refusal, digest, fingerprint


class Snapshot:
    def __init__(self, configuration, files):
        self.files = dict(files)
        self.manifest = {
            "contract": "plumb.blob-source/v4",
            "configuration": configuration,
            "references": [{"path": path, "mode": mode, "digest": digest(body)}
                           for path, (mode, body) in sorted(self.files.items())]}
        self.key = fingerprint(self.manifest)

    def materialize(self, destination):
        destination = Path(destination)
        if destination.exists() or destination.is_symlink():
            raise Refusal("input materialization requires a new destination")
        for name, (mode, body) in self.files.items():
            self.path(name)
            if mode not in ("100644", "100755", "120000"):
                raise Refusal("unsupported input file mode")
            if mode == "120000":
                self.link(name, body)
        destination.mkdir(parents=False)
        for name, (mode, body) in sorted(self.files.items()):
            path = destination.joinpath(*PurePosixPath(name).parts)
            path.parent.mkdir(parents=True, exist_ok=True)
            if mode == "120000":
                path.symlink_to(body.decode("utf-8"))
            else:
                with path.open("xb") as output:
                    output.write(body)
                path.chmod(0o755 if mode == "100755" else 0o644)
        self.verify(destination)
        return self.key

    def verify(self, destination):
        destination = Path(destination)
        if destination.is_symlink() or not destination.is_dir():
            raise Refusal("materialized input is not a directory")
        found = set()
        for root, directories, files in os.walk(destination, followlinks=False):
            links = [name for name in directories if (Path(root) / name).is_symlink()]
            directories[:] = [name for name in directories if name not in links]
            for name in files + links:
                path = Path(root) / name
                relative = path.relative_to(destination).as_posix()
                found.add(relative)
                if relative not in self.files:
                    raise Refusal("materialized input carries an undeclared file")
                mode, expected = self.files[relative]
                metadata = path.lstat()
                if mode == "120000":
                    if not stat.S_ISLNK(metadata.st_mode):
                        raise Refusal("materialized symlink changed kind")
                    try:
                        if not path.resolve().is_relative_to(destination.resolve()):
                            raise Refusal("materialized symlink escapes its source")
                    except RuntimeError as error:
                        raise Refusal("materialized symlink is cyclic") from error
                    actual = os.readlink(path).encode("utf-8")
                else:
                    if not stat.S_ISREG(metadata.st_mode):
                        raise Refusal("materialized input changed kind")
                    if os.name != "nt" and bool(metadata.st_mode & 0o111) != (mode == "100755"):
                        raise Refusal("materialized input changed executable mode")
                    actual = path.read_bytes()
                if actual != expected:
                    raise Refusal("materialized input differs from its hashed bytes")
        if found != set(self.files):
            raise Refusal("materialized input is incomplete")

    def path(self, name):
        path = PurePosixPath(name)
        if path.is_absolute() or any(part in ("", ".", "..") for part in name.split("/")):
            raise Refusal("input path escapes its destination")
        if "\\" in name or ":" in name or any(part.lower() == ".git" for part in path.parts):
            raise Refusal("input path is not a portable source path")
        for parent in path.parents:
            if parent.as_posix() in self.files:
                raise Refusal("input file is also a parent directory")

    def link(self, name, body):
        try:
            target = body.decode("utf-8")
        except UnicodeError as error:
            raise Refusal("input symlink is not UTF-8") from error
        if not target or PurePosixPath(target).is_absolute() or "\\" in target or ":" in target:
            raise Refusal("input symlink must remain within its source")
        parts = list(PurePosixPath(name).parent.parts)
        for part in target.split("/"):
            if part == "..":
                if not parts:
                    raise Refusal("input symlink escapes its source")
                parts.pop()
            elif part not in ("", "."):
                parts.append(part)
