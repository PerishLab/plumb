from .blob import Refusal, name, object_fields, sha


class Declaration:
    def __init__(self, value):
        object_fields(value, ("schema", "nodes", "targets"))
        if value["schema"] != "plumb.blob-graph/v1":
            raise Refusal("unsupported blob graph schema")
        if not isinstance(value["nodes"], list) or not value["nodes"]:
            raise Refusal("graph needs nodes")
        self.nodes = {}
        for node in value["nodes"]:
            self.add(node)
        if not isinstance(value["targets"], list) or not value["targets"]:
            raise Refusal("graph needs targets")
        self.targets = value["targets"]
        for target in self.targets:
            name(target)
        if len(set(self.targets)) != len(self.targets):
            raise Refusal("duplicate target")
        for node in self.nodes.values():
            for reference in self.references(node):
                self.reference(reference)
        self.order = self.ordered()

    def add(self, node):
        object_fields(node, ("id", "inputs", "outputs", "execution"), ("requires", "prepare", "evidence"))
        identity = name(node["id"])
        if identity in self.nodes:
            raise Refusal("duplicate node")
        inputs = node["inputs"]
        if not isinstance(inputs, dict) or "implementation" not in inputs:
            raise Refusal("node inputs must bind its implementation")
        for key, value in inputs.items():
            name(key)
            if isinstance(value, str):
                sha(value)
            else:
                object_fields(value, ("node", "output"))
        outputs = node["outputs"]
        if not isinstance(outputs, list) or not outputs:
            raise Refusal("node needs unique output names")
        for output in outputs:
            name(output)
        if len(set(outputs)) != len(outputs):
            raise Refusal("node needs unique output names")
        evidence = node.get("evidence", [])
        if not isinstance(evidence, list):
            raise Refusal("evidence must be a name list")
        for label in evidence:
            name(label)
        if len(set(evidence)) != len(evidence):
            raise Refusal("duplicate evidence name")
        object_fields(node["execution"], ("entry", "capability"), ("payload", "materialize"))
        materialize = node["execution"].get("materialize", [])
        if not isinstance(materialize, list) or any(not isinstance(label, str) for label in materialize):
            raise Refusal("materialized inputs must be a label list")
        if len(set(materialize)) != len(materialize) or set(materialize) - set(inputs):
            raise Refusal("materialization selects duplicate or unknown inputs")
        name(node["execution"]["entry"])
        name(node["execution"]["capability"])
        for relation in ("requires", "prepare"):
            if not isinstance(node.get(relation, []), list):
                raise Refusal(relation + " must be a list")
        self.nodes[identity] = node

    def references(self, node):
        inputs = [value for value in node["inputs"].values() if isinstance(value, dict)]
        return inputs + node.get("requires", []) + node.get("prepare", [])

    def reference(self, reference):
        object_fields(reference, ("node", "output"))
        node = self.nodes.get(name(reference["node"]))
        if node is None or reference["output"] not in node["outputs"]:
            raise Refusal("reference names an unknown node output")

    def ordered(self):
        order, active, done = [], set(), set()

        def visit(identity):
            if identity in active:
                raise Refusal("cyclic blob graph")
            if identity in done:
                return
            if identity not in self.nodes:
                raise Refusal("unknown graph target")
            active.add(identity)
            for reference in self.references(self.nodes[identity]):
                visit(reference["node"])
            active.remove(identity)
            done.add(identity)
            order.append(identity)

        for identity in self.nodes:
            visit(identity)
        reachable = set()

        def select(identity):
            if identity in reachable:
                return
            reachable.add(identity)
            for reference in self.references(self.nodes[identity]):
                select(reference["node"])

        for identity in self.targets:
            if identity not in self.nodes:
                raise Refusal("unknown graph target")
            select(identity)
        return [identity for identity in order if identity in reachable]
