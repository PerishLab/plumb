import stat
from pathlib import Path

from .blob import Refusal, digest, object_fields, sha
from .completion import complete


def regular(path):
    path = Path(path)
    if not stat.S_ISREG(path.lstat().st_mode):
        raise Refusal("handoff content must be a regular file")
    return path.read_bytes()


def pack(result, destination):
    object_fields(result, ("key", "outputs"), ("evidence",))
    sha(result["key"])
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    manifest = {"schema": "plumb.blob-handoff/v1", "key": result["key"]}
    for kind in ("outputs", "evidence"):
        files = result.get(kind, {})
        if not isinstance(files, dict):
            raise Refusal("handoff results require named files")
        manifest[kind] = {}
        for label, path in files.items():
            if not isinstance(path, str):
                raise Refusal("handoff results require local file paths")
            body = regular(path)
            value = digest(body)
            target = destination / value
            if not target.exists():
                with target.open("xb") as output:
                    output.write(body)
            elif regular(target) != body:
                raise Refusal("handoff contains conflicting blob bytes")
            manifest[kind][label] = value
    return manifest


def inspect(manifest, root):
    object_fields(manifest, ("schema", "key", "outputs", "evidence"))
    if manifest["schema"] != "plumb.blob-handoff/v1":
        raise Refusal("unsupported blob handoff")
    sha(manifest["key"])
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise Refusal("handoff root must be an owned directory")
    result = {"key": manifest["key"]}
    for kind in ("outputs", "evidence"):
        if not isinstance(manifest[kind], dict):
            raise Refusal("handoff references must be a digest map")
        result[kind] = {}
        for label, value in manifest[kind].items():
            path = root / sha(value)
            if digest(regular(path)) != value:
                raise Refusal("handoff blob differs from its declared digest")
            result[kind][label] = str(path)
    return result


def record(graph, node, manifest, root, inventory):
    result = inspect(manifest, root)
    return complete(graph, node, result, inventory)
