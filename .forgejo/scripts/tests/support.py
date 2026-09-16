import copy

from lib.blob import Refusal, digest
from lib.declaration import Declaration
from lib.inventory import Inventory
from lib.planner import plan


class Memory:
    def __init__(self):
        self.objects = {}
        self.writes = []

    def read(self, key):
        return self.objects.get(key)

    def create(self, key, body):
        self.writes.append(key)
        if key in self.objects:
            return False
        self.objects[key] = body
        return True

    def verify(self, key, expected):
        body = self.objects.get(key)
        if body is None or digest(body) != expected:
            raise Refusal("missing or corrupt blob")


def reference(node, output):
    return {"node": node, "output": output}


def node(identity, inputs, output, capability="native", requires=()):
    return {"id": identity, "inputs": {"implementation": digest(identity.encode()), **inputs},
            "outputs": [output], "requires": list(requires),
            "execution": {"entry": "fixture." + identity, "capability": capability}}


def declaration():
    return {"schema": "plumb.blob-graph/v1", "nodes": [
        node("produce", {"source": digest(b"source-A"), "tool": digest(b"tool-A")}, "content"),
        node("check", {"content": reference("produce", "content")}, "receipt"),
        node("bind", {"content": reference("produce", "content"),
                      "identity": digest(b"identity-A")}, "bound"),
        node("deliver", {"content": reference("bind", "bound"),
                         "destination": digest(b"destination-A")}, "receipt", "network",
             [reference("check", "receipt")]),
        node("independent", {"source": digest(b"web-A")}, "receipt", "network"),
    ], "targets": ["deliver", "independent"]}


class Fixture:
    def __init__(self):
        self.value = declaration()
        self.store = Memory()
        self.inventory = Inventory(self.store)

    def plan(self):
        return plan(Declaration(copy.deepcopy(self.value)), self.inventory)

    def finish(self, identity, body=None):
        current = self.plan()["nodes"][identity]
        assert current["state"] == "run"
        definition = next(node for node in self.value["nodes"] if node["id"] == identity)
        output = self.inventory.upload(body if body is not None else identity.encode())
        self.inventory.complete(current["key"], {definition["outputs"][0]: output})

    def seed(self):
        while not self.plan()["complete"]:
            for identity in self.plan()["run"]:
                self.finish(identity)
        self.store.writes.clear()

    def change(self, identity, field, body):
        definition = next(node for node in self.value["nodes"] if node["id"] == identity)
        definition["inputs"][field] = digest(body)
