from .acceptance import collect, declaration
from .blob import Refusal
from .handoff import inspect, record
from .workflow import observe, read


def receive(value, contract, artifacts, inventory, reader=read):
    """Caller supplies trusted intent/contract and freshly downloaded artifact roots.

    Artifact keys are graph node identities, never paths supplied by a producer.
    No artifact is executed. Shared records are recomputed, not imported.
    """
    graph = declaration(value, contract)
    observed = observe(value, contract, reader)
    if not isinstance(artifacts, dict) or set(artifacts) - set(graph.order):
        raise Refusal("acceptance artifacts name undeclared work")
    for identity, artifact in artifacts.items():
        if not isinstance(artifact, tuple) or len(artifact) != 2:
            raise Refusal("collector requires a manifest and an owned artifact root")
        manifest, root = artifact
        result = inspect(manifest, root)
        node = graph.nodes[identity]
        if (set(result["outputs"]) != set(node["outputs"])
                or set(result["evidence"]) != set(node.get("evidence", []))):
            raise Refusal("acceptance handoff differs from the declared output contract")
    # The graph's existing topological order owns completion dependencies.
    # A failed later transaction leaves only verified reusable predecessors.
    for identity in graph.order:
        if identity in artifacts:
            manifest, root = artifacts[identity]
            record(graph, identity, manifest, root, inventory)
    return collect(value, contract, inventory, observed)
