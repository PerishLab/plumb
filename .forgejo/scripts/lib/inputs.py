import tempfile
from pathlib import Path

from .blob import Refusal, digest


class Inputs:
    def __init__(self, snapshots):
        self.snapshots = snapshots

    @classmethod
    def select(cls, declaration, identity, source):
        node = next((node for node in declaration["nodes"] if node["id"] == identity), None)
        if node is None:
            raise Refusal("execution input node is not declared")
        snapshots = {}
        for label in node["execution"].get("materialize", []):
            value = node["inputs"].get(label)
            if not isinstance(value, dict) or "tree" not in value:
                raise Refusal("materialization requires a declared tree input")
            snapshots[label] = source.snapshot(value["tree"])
        return cls(snapshots)

    def materialize(self, request, seat):
        for label, snapshot in self.snapshots.items():
            if request.get("inputs", {}).get(label, {}).get("digest") != snapshot.key:
                raise Refusal("execution input differs from its planned key")
        if not self.snapshots:
            return {}
        attempt = Path(tempfile.mkdtemp(prefix="inputs-", dir=seat))
        result = {}
        for label, snapshot in self.snapshots.items():
            destination = attempt / digest(label.encode())
            snapshot.materialize(destination)
            result[label] = {"key": snapshot.key, "root": str(destination.resolve())}
        return result
