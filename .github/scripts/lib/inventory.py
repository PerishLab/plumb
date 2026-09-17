from .blob import Conflict, Refusal, Unknown, decode, digest, encode, object_fields, sha


class Inventory:
    def __init__(self, store):
        self.store = store
        self.verified = set()

    def blob(self, value):
        return "v2/blobs/sha256/" + sha(value)

    def record(self, key):
        return "v2/records/" + sha(key) + ".json"

    def verify(self, value):
        if value in self.verified:
            return
        self.store.verify(self.blob(value), value)
        self.verified.add(value)

    def lookup(self, key, outputs, evidence=()):
        body = self.store.read(self.record(key))
        if body is None:
            return None
        return self.validate(body, key, outputs, evidence)

    def validate(self, body, key, outputs, evidence):
        record = decode(body)
        object_fields(record, ("schema", "key", "outputs", "evidence"))
        if record["schema"] != "plumb.blob-record/v1" or record["key"] != key:
            raise Refusal("completion record identity mismatch")
        if not isinstance(record["outputs"], dict) or set(record["outputs"]) != set(outputs):
            raise Refusal("completion output contract mismatch")
        if not isinstance(record["evidence"], dict) or set(record["evidence"]) != set(evidence):
            raise Refusal("completion evidence contract mismatch")
        for value in [*record["outputs"].values(), *record["evidence"].values()]:
            self.verify(sha(value))
        return record

    def complete(self, key, outputs, evidence=None):
        evidence = evidence or {}
        if not outputs:
            raise Refusal("completion requires an output or receipt blob")
        for value in [*outputs.values(), *evidence.values()]:
            self.verify(sha(value))
        record = {"schema": "plumb.blob-record/v1", "key": sha(key),
                  "outputs": outputs, "evidence": evidence}
        body = encode(record)
        route = self.record(key)
        uncertainty = None
        try:
            self.store.create(route, body)
        except Unknown as error:
            uncertainty = error
        held = self.store.read(route)
        if held is None:
            if uncertainty:
                raise uncertainty
            raise Refusal("completion is not readable after conditional creation")
        accepted = self.validate(held, key, outputs, evidence)
        if accepted["outputs"] != outputs:
            raise Conflict("action key has conflicting completion evidence")
        return accepted

    def upload(self, body):
        value = digest(body)
        route = self.blob(value)
        try:
            self.store.create(route, body)
        except Unknown:
            self.store.verify(route, value)
        self.verify(value)
        return value

    def download(self, value):
        route = self.blob(value)
        body = self.store.read(route)
        if body is None:
            raise Refusal("referenced blob is absent")
        if digest(body) != value:
            raise Refusal("referenced blob content differs from its SHA-256")
        return body
