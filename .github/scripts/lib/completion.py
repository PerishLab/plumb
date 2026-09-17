from pathlib import Path

from .blob import Refusal, object_fields, sha
from .execution import closure
from .planner import plan


def complete(graph, identity, result, inventory):
    object_fields(result, ("key", "outputs"), ("evidence",))
    sha(result["key"])
    files = result["outputs"]
    if not isinstance(files, dict) or any(not isinstance(path, str) for path in files.values()):
        raise Refusal("executor outputs must name local files")
    if identity not in graph.nodes or set(files) != set(graph.nodes[identity]["outputs"]):
        raise Refusal("executor result does not match the declared node outputs")
    current = plan(closure(graph, identity), inventory)["nodes"].get(identity)
    if current is None or current["key"] != result["key"] or current["waiting"]:
        raise Refusal("cannot complete a node with unresolved prerequisites")
    evidence = result.get("evidence", {})
    if not isinstance(evidence, dict) or any(not isinstance(path, str) for path in evidence.values()):
        raise Refusal("executor evidence must name local files")
    if set(evidence) != set(graph.nodes[identity].get("evidence", [])):
        raise Refusal("executor result does not match the declared evidence")
    outputs = {label: inventory.upload(Path(path).read_bytes()) for label, path in files.items()}
    proofs = {label: inventory.upload(Path(path).read_bytes()) for label, path in evidence.items()}
    return inventory.complete(current["key"], outputs, proofs)
