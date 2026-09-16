import unittest

from lib.blob import Conflict, Refusal, Unknown, digest
from lib.inventory import Inventory
from support import Fixture


class Planning(unittest.TestCase):
    def setUp(self):
        self.fixture = Fixture()
        self.fixture.seed()

    def test_same_request_starts_no_business_job(self):
        self.assertTrue(self.fixture.plan()["complete"])
        self.assertEqual(self.fixture.plan()["run"], [])
        self.assertEqual(self.fixture.store.writes, [])

    def test_identity_change_only_binds_then_delivers(self):
        self.fixture.change("bind", "identity", b"identity-B")
        self.assertEqual(self.fixture.plan()["run"], ["bind"])
        self.fixture.finish("bind", b"bound-B")
        self.assertEqual(self.fixture.plan()["run"], ["deliver"])

    def test_source_change_defers_consumers_until_output_known(self):
        self.fixture.change("produce", "source", b"source-B")
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["produce"])
        self.assertIsNone(held["nodes"]["bind"]["key"])
        self.assertEqual(held["nodes"]["independent"]["state"], "reuse")

    def test_same_bytes_after_rebuild_reuse_consumers(self):
        self.fixture.change("produce", "source", b"source-B")
        self.fixture.finish("produce")
        self.assertTrue(self.fixture.plan()["complete"])

    def test_validation_change_does_not_republish(self):
        self.fixture.change("check", "implementation", b"check-B")
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["check"])
        self.assertEqual(held["nodes"]["deliver"]["state"], "reuse")
        self.assertFalse(held["nodes"]["deliver"]["complete"])
        self.assertFalse(held["complete"])
        self.fixture.finish("check", b"check-B")
        self.assertTrue(self.fixture.plan()["complete"])

    def test_missing_completion_only_runs_affected_executor(self):
        key = self.fixture.plan()["nodes"]["deliver"]["key"]
        del self.fixture.store.objects[self.fixture.inventory.record(key)]
        self.assertEqual(self.fixture.plan()["run"], ["deliver"])

    def test_unknown_inventory_refuses_instead_of_running(self):
        def unknown(key):
            raise Unknown("transport")
        self.fixture.store.read = unknown
        with self.assertRaises(Unknown):
            self.fixture.plan()
        self.assertEqual(self.fixture.store.writes, [])

    def test_corruption_is_not_a_cache_miss(self):
        route = self.fixture.inventory.blob(digest(b"produce"))
        self.fixture.store.objects[route] = b"corrupt"
        self.fixture.inventory = Inventory(self.fixture.store)
        with self.assertRaises(Refusal):
            self.fixture.plan()
        self.assertEqual(self.fixture.store.writes, [])

    def test_conflicting_writers_never_replace_a_result(self):
        key = self.fixture.plan()["nodes"]["produce"]["key"]
        other = self.fixture.inventory.upload(b"different")
        with self.assertRaises(Conflict):
            self.fixture.inventory.complete(key, {"content": other})
        self.assertEqual(self.fixture.plan()["nodes"]["produce"]["outputs"]["content"],
                         digest(b"produce"))

    def test_fresh_runner_empty_plan_needs_no_business_tool(self):
        self.fixture.inventory = Inventory(self.fixture.store)
        self.assertTrue(self.fixture.plan()["complete"])
        self.assertEqual(self.fixture.plan()["run"], [])

    def test_missing_bind_only_requires_its_declared_capability(self):
        key = self.fixture.plan()["nodes"]["bind"]["key"]
        del self.fixture.store.objects[self.fixture.inventory.record(key)]
        self.fixture.inventory = Inventory(self.fixture.store)
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["bind"])
        self.assertEqual(held["nodes"]["bind"]["execution"]["capability"], "native")
        self.fixture.finish("bind")
        self.assertTrue(self.fixture.plan()["complete"])

    def test_unrelated_input_change_does_not_invalidate_other_nodes(self):
        self.fixture.change("independent", "source", b"web-B")
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["independent"])
        for identity in ("produce", "check", "bind", "deliver"):
            self.assertTrue(held["nodes"][identity]["complete"])


if __name__ == "__main__":
    unittest.main()
