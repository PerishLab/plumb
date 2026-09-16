import copy
import subprocess
from pathlib import PurePosixPath

from .blob import Refusal, fingerprint, object_fields
from .document import project
from .patch import apply
from .snapshot import Snapshot


def relative(value):
    if not isinstance(value, str) or not value or "\\" in value:
        raise Refusal("input path must be relative POSIX text")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in (".", "..") for part in value.split("/")):
        raise Refusal("input path escapes its source root")
    return value


class Source:
    def __init__(self, root):
        self.root = root
        self.leaves = None
        self.bodies = {}

    def git(self, arguments, body=None):
        result = subprocess.run(["git", "-C", str(self.root), *arguments],
                                input=body, capture_output=True, timeout=30)
        if result.returncode:
            raise Refusal("cannot read the exact source tree")
        return result.stdout

    def load(self):
        if self.leaves is not None:
            return
        self.leaves = {}
        for row in self.git(["ls-tree", "-r", "-z", "HEAD"]).split(b"\0"):
            if not row:
                continue
            metadata, path = row.split(b"\t", 1)
            mode, kind, oid = metadata.decode("ascii").split()
            self.leaves[path.decode("utf-8")] = (mode, kind, oid)

    def read(self, paths):
        wanted = {self.leaves[path][2] for path in paths} - self.bodies.keys()
        if not wanted:
            return
        body = self.git(["cat-file", "--batch"], "".join(oid + "\n" for oid in sorted(wanted)).encode())
        offset = 0
        for oid in sorted(wanted):
            end = body.index(b"\n", offset)
            actual, kind, length = body[offset:end].split()
            size = int(length)
            if actual.decode() != oid or kind != b"blob":
                raise Refusal("source input is not a Git blob")
            offset = end + 1
            self.bodies[oid] = body[offset:offset + size]
            offset += size + 1
        if offset != len(body):
            raise Refusal("invalid Git blob stream")

    def fingerprint(self, recipe):
        return self.snapshot(recipe).key

    def snapshot(self, recipe):
        object_fields(recipe, ("paths",), ("projects", "source", "patches"))
        if not isinstance(recipe["paths"], list) or not recipe["paths"]:
            raise Refusal("source input needs explicit paths")
        roots = [relative(path) for path in recipe["paths"]]
        projects = recipe.get("projects", {})
        if not isinstance(projects, dict):
            raise Refusal("source projects must be a path map")
        patches = recipe.get("patches", {})
        if not isinstance(patches, dict) or set(patches) - set(projects):
            raise Refusal("prepared patches require declared source projections")
        self.load()
        paths = sorted(path for path in self.leaves if any(
            path == root or path.startswith(root.rstrip("/") + "/") for root in roots))
        if not paths:
            raise Refusal("source input selects no files")
        if set(projects) - set(paths):
            raise Refusal("projection lies outside selected source files")
        if any(self.leaves[path][1] != "blob" for path in paths):
            raise Refusal("submodules need an explicit resolved input")
        self.read(paths)
        files = {}
        for path in paths:
            mode, _, oid = self.leaves[path]
            body = self.bodies[oid]
            if path in projects:
                if mode not in ("100644", "100755"):
                    raise Refusal("cannot project a symlink")
                body = apply(body, patches[path]) if path in patches else project(body, projects[path])
            files[path] = (mode, body)
        configuration = {"paths": sorted(set(roots)), "projects": copy.deepcopy(projects)}
        if "source" in recipe:
            configuration["source"] = recipe["source"]
        return Snapshot(configuration, files)


class Sources:
    def __init__(self, roots):
        self.sources = {name: Source(root) for name, root in roots.items() if root is not None}

    def snapshot(self, recipe):
        name = recipe.get("source", "product")
        if not isinstance(name, str) or name not in self.sources:
            raise Refusal("input names an unavailable source")
        return self.sources[name].snapshot(recipe)

    def fingerprint(self, recipe):
        return self.snapshot(recipe).key


def resolve(value, source):
    held = copy.deepcopy(value)
    if not isinstance(held, dict) or not isinstance(held.get("nodes"), list):
        raise Refusal("invalid declaration")
    for node in held["nodes"]:
        if not isinstance(node, dict) or not isinstance(node.get("inputs"), dict):
            raise Refusal("invalid node inputs")
        for label, field in node["inputs"].items():
            if isinstance(field, dict) and "tree" in field:
                object_fields(field, ("tree",))
                node["inputs"][label] = source.fingerprint(field["tree"])
            elif isinstance(field, dict) and "value" in field:
                object_fields(field, ("value",))
                node["inputs"][label] = fingerprint(field["value"])
    return held
