from urllib.parse import urlsplit

from .blob import Refusal
from .declaration import Declaration
from .planner import plan


def closure(graph, identity):
    if identity not in graph.order:
        raise Refusal("execution node is outside the declared target closure")
    return Declaration({"schema": "plumb.blob-graph/v1",
                        "nodes": list(graph.nodes.values()), "targets": [identity]})


class Execution:
    def __init__(self, graph, inventory, origin):
        address = urlsplit(origin)
        if address.scheme != "https" or not address.hostname or address.username or address.password:
            raise Refusal("blob delivery requires an uncredentialed HTTPS origin")
        if address.query or address.fragment or address.path not in ("", "/"):
            raise Refusal("blob delivery requires an origin without a route")
        if any(character.isspace() or ord(character) < 32 for character in origin):
            raise Refusal("invalid blob delivery origin")
        self.graph = graph
        self.inventory = inventory
        self.origin = origin.rstrip("/")

    def carrier(self, digest):
        return {"type": "workload", "source": self.origin + "/" + self.inventory.blob(digest)}

    def prepare(self, identity):
        graph = closure(self.graph, identity)
        planned = plan(graph, self.inventory)
        current = planned["nodes"][identity]
        if current["waiting"]:
            preparation = {ref["node"] for ref in graph.nodes[identity].get("prepare", [])}
            if current["state"] != "reuse" and current["key"] is not None and set(current["waiting"]) <= preparation:
                return {"state": "prepare", "nodes": current["waiting"]}
            raise Refusal("execution prerequisites are unresolved")
        if current["complete"]:
            return {"state": "reuse", "completion": current}
        node = graph.nodes[identity]
        inputs = {}
        for label, value in node["inputs"].items():
            if isinstance(value, str):
                inputs[label] = {"digest": value, "reuse": {"type": "none", "source": ""}}
            else:
                digest = planned["nodes"][value["node"]]["outputs"][value["output"]]
                inputs[label] = {"digest": digest, "reuse": self.carrier(digest)}
        producers = {}
        for reference in graph.references(node):
            producer = reference["node"]
            held = planned["nodes"][producer]
            producers[producer] = {
                kind: {label: {"digest": digest, "reuse": self.carrier(digest)}
                       for label, digest in held[kind].items()}
                for kind in ("outputs", "evidence")}
        return {"state": "run", "request": {
            "schema": "plumb.blob-execution/v1", "node": identity,
            "key": current["key"], "entry": node["execution"]["entry"],
            "inputs": inputs, "producers": producers,
            "preparation": node.get("prepare", []),
            "payload": node["execution"].get("payload")}}
