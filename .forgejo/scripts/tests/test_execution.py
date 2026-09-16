import unittest

from lib.blob import Refusal, digest
from lib.declaration import Declaration
from lib.execution import Execution
from support import Fixture


class Requests(unittest.TestCase):
    def test_opaque_payload_is_transported_not_implicitly_hashed(self):
        fixture = Fixture()
        node = fixture.value["nodes"][0]
        node["execution"]["payload"] = {"marker": "beta.1", "operation": "opaque"}
        first = self.prepare(fixture, "produce")["request"]
        node["execution"]["payload"] = {"marker": "beta.2", "operation": "opaque"}
        second = self.prepare(fixture, "produce")["request"]
        self.assertEqual(first["key"], second["key"])
        self.assertEqual(second["payload"], node["execution"]["payload"])
        node["inputs"]["identity"] = digest(b"beta.2")
        self.assertNotEqual(first["key"], self.prepare(fixture, "produce")["request"]["key"])

    def prepare(self, fixture, identity):
        return Execution(Declaration(fixture.value), fixture.inventory,
                         "https://blob.example").prepare(identity)

    def test_cold_node_has_no_fabricated_blob(self):
        held = self.prepare(Fixture(), "produce")
        self.assertEqual(held["state"], "run")
        self.assertEqual(held["request"]["inputs"]["source"]["reuse"],
                         {"type": "none", "source": ""})

    def test_unresolved_producer_cannot_execute(self):
        with self.assertRaises(Refusal):
            self.prepare(Fixture(), "bind")

    def test_proven_content_is_carried_by_digest(self):
        fixture = Fixture()
        fixture.finish("produce")
        held = self.prepare(fixture, "bind")["request"]
        content = held["inputs"]["content"]
        self.assertEqual(content["digest"], digest(b"produce"))
        self.assertEqual(content["reuse"], {"type": "workload", "source":
                         "https://blob.example/v2/blobs/sha256/" + digest(b"produce")})

    def test_warm_node_needs_no_executor(self):
        fixture = Fixture()
        fixture.seed()
        held = self.prepare(fixture, "deliver")
        self.assertEqual(held["state"], "reuse")
        self.assertNotIn("request", held)

    def test_reused_publication_does_not_bypass_changed_check(self):
        fixture = Fixture()
        fixture.seed()
        fixture.change("check", "implementation", b"different")
        with self.assertRaises(Refusal):
            self.prepare(fixture, "deliver")

    def test_unrelated_corruption_does_not_block_local_work(self):
        fixture = Fixture()
        fixture.seed()
        fixture.change("bind", "implementation", b"different")
        key = fixture.plan()["nodes"]["independent"]["key"]
        fixture.store.objects[fixture.inventory.record(key)] = b"corrupt"
        self.assertEqual(self.prepare(fixture, "bind")["state"], "run")

    def test_outside_target_closure_refuses(self):
        fixture = Fixture()
        fixture.value["targets"] = ["independent"]
        with self.assertRaises(Refusal):
            self.prepare(fixture, "produce")

    def test_producer_evidence_is_available_to_business_executor(self):
        fixture = Fixture()
        fixture.value["nodes"][0]["evidence"] = ["production"]
        key = fixture.plan()["nodes"]["produce"]["key"]
        proof = fixture.inventory.upload(b"business proof")
        fixture.inventory.complete(key, {"content": fixture.inventory.upload(b"content")},
                                   {"production": proof})
        held = self.prepare(fixture, "bind")["request"]
        self.assertEqual(held["producers"]["produce"]["evidence"]["production"]["digest"], proof)
        self.assertNotIn("production", held["inputs"])

    def test_unsafe_origins_refuse(self):
        fixture = Fixture()
        for origin in ("http://example", "https://user:pass@example", "https://example/route",
                       "https://example?token=secret", "https://example#fragment",
                       "https://exa mple", "https://example\n"):
            with self.subTest(origin=origin), self.assertRaises(Refusal):
                Execution(Declaration(fixture.value), fixture.inventory, origin)


if __name__ == "__main__":
    unittest.main()
