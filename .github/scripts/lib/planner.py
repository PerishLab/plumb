from .blob import fingerprint


def action_key(node, inputs):
    return fingerprint({
        "contract": "plumb.blob-key/v3",
        "configuration": {"inputs": sorted(node["inputs"]), "outputs": sorted(node["outputs"]),
                          "evidence": sorted(node.get("evidence", [])),
                          "materialize": sorted(node["execution"].get("materialize", []))},
        "references": inputs,
    })


def plan(graph, inventory):
    nodes = {}
    def demand(identity):
        if identity in nodes:
            return nodes[identity]
        node = graph.nodes[identity]
        inputs = {}
        pending = []
        for label, value in node["inputs"].items():
            if isinstance(value, str):
                inputs[label] = value
            else:
                producer = demand(value["node"])
                if producer["complete"]:
                    inputs[label] = producer["outputs"][value["output"]]
                else:
                    pending.append(value["node"])
        prerequisites = [ref["node"] for ref in node.get("requires", [])
                         if not demand(ref["node"])["complete"]]
        key = None if pending else action_key(node, inputs)
        completion = None if key is None else inventory.lookup(
            key, node["outputs"], node.get("evidence", []))
        outputs = None if completion is None else completion["outputs"]
        if key is not None and outputs is None:
            prerequisites += [ref["node"] for ref in node.get("prepare", [])
                              if not demand(ref["node"])["complete"]]
        state = "pending" if pending or prerequisites else "run"
        if outputs is not None:
            state = "reuse"
        nodes[identity] = {
            "key": key, "state": state, "complete": outputs is not None and not prerequisites,
            "outputs": outputs or {}, "waiting": sorted(set(pending + prerequisites)),
            "evidence": {} if completion is None else completion["evidence"],
            "execution": node["execution"],
        }
        return nodes[identity]

    for target in graph.targets:
        demand(target)
    nodes = {identity: nodes.get(identity, {
        "key": None, "state": "skip", "complete": False, "outputs": {},
        "waiting": [], "evidence": {}, "execution": graph.nodes[identity]["execution"],
    }) for identity in graph.order}
    return {"schema": "plumb.blob-plan/v1", "nodes": nodes,
            "complete": all(nodes[target]["complete"] for target in graph.targets),
            "run": [identity for identity, node in nodes.items() if node["state"] == "run"]}
