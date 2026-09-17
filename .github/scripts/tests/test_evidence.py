import unittest

from lib.blob import Conflict, Refusal, digest
from lib.declaration import Declaration
from lib.inventory import Inventory
from lib.planner import plan
from support import Fixture, Memory


class Evidence(unittest.TestCase):
    def setUp(self):
        self.store = Memory()
        self.inventory = Inventory(self.store)
        self.key = digest(b"action")
        self.content = self.inventory.upload(b"artifact")

    def test_different_observation_bytes_preserve_first_completion(self):
        first = self.inventory.upload(b"validated at runner location A")
        second = self.inventory.upload(b"validated at runner location B")
        held = self.inventory.complete(self.key, {"content": self.content}, {"production": first})
        retry = self.inventory.complete(self.key, {"content": self.content}, {"production": second})
        self.assertEqual(held, retry)
        self.assertEqual(retry["evidence"]["production"], first)

    def test_evidence_does_not_hide_conflicting_output(self):
        proof = self.inventory.upload(b"observation")
        self.inventory.complete(self.key, {"content": self.content}, {"production": proof})
        changed = self.inventory.upload(b"different artifact")
        with self.assertRaises(Conflict):
            self.inventory.complete(self.key, {"content": changed}, {"production": proof})

    def test_missing_required_evidence_never_reuses(self):
        self.inventory.complete(self.key, {"content": self.content})
        with self.assertRaises(Refusal):
            self.inventory.lookup(self.key, ["content"], ["production"])

    def test_corrupt_evidence_never_reuses(self):
        proof = self.inventory.upload(b"observation")
        self.inventory.complete(self.key, {"content": self.content}, {"production": proof})
        self.store.objects[self.inventory.blob(proof)] = b"corrupt"
        with self.assertRaises(Refusal):
            Inventory(self.store).lookup(self.key, ["content"], ["production"])

    def test_declared_evidence_is_available_but_not_a_consumer_content_input(self):
        fixture = Fixture()
        fixture.value["nodes"][0]["evidence"] = ["production"]
        graph = Declaration(fixture.value)
        pending = plan(graph, fixture.inventory)
        proof = fixture.inventory.upload(b"producer observation")
        content = fixture.inventory.upload(b"produce")
        fixture.inventory.complete(pending["nodes"]["produce"]["key"],
                                   {"content": content}, {"production": proof})
        held = plan(graph, fixture.inventory)
        self.assertEqual(held["nodes"]["produce"]["evidence"], {"production": proof})
        self.assertEqual(held["nodes"]["check"]["state"], "run")


if __name__ == "__main__":
    unittest.main()
