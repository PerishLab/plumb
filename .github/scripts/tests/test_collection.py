import copy
import tempfile
import unittest
from pathlib import Path

from lib.blob import Refusal
from lib.collection import receive
from lib.handoff import pack
from support import Fixture
import test_acceptance
import test_workflow


class Collection(unittest.TestCase):
    def setUp(self):
        acceptance = test_acceptance.Acceptance()
        acceptance.setUp()
        self.value, self.contract = acceptance.value, acceptance.contract
        self.fixture = acceptance.fixture
        self.remote = test_workflow.Workflow()
        self.remote.setUp()
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def artifacts(self):
        # A candidate has its own provisional state, not the shared writer.
        candidate = Fixture()
        artifacts = {}
        while not candidate.plan()["complete"]:
            for identity in candidate.plan()["run"]:
                key = candidate.plan()["nodes"][identity]["key"]
                node = next(node for node in candidate.value["nodes"] if node["id"] == identity)
                path = self.root / (identity + ".bin")
                path.write_bytes(identity.encode())
                result = {"key": key, "outputs": {node["outputs"][0]: str(path)}}
                seat = self.root / identity
                artifacts[identity] = (pack(result, seat), seat)
                candidate.finish(identity)
        return artifacts

    def test_cold_handoff_then_warm_reuse(self):
        artifacts = self.artifacts()
        self.assertEqual(self.fixture.store.writes, [])
        receipt = receive(self.value, self.contract, artifacts,
                          self.fixture.inventory, self.remote.reader)
        self.assertTrue(self.fixture.plan()["complete"])
        self.fixture.store.writes.clear()
        reused = receive(self.value, self.contract, {}, self.fixture.inventory, self.remote.reader)
        self.assertEqual(receipt, reused)
        self.assertEqual(self.fixture.store.writes, [])

    def test_failed_execution_cannot_write_even_valid_artifacts(self):
        artifacts = self.artifacts()
        self.remote.jobs[0]["conclusion"] = "failure"
        with self.assertRaises(Refusal):
            receive(self.value, self.contract, artifacts, self.fixture.inventory, self.remote.reader)
        self.assertEqual(self.fixture.store.writes, [])

    def test_invalid_later_artifact_is_preflighted_before_writes(self):
        artifacts = self.artifacts()
        manifest, root = artifacts["deliver"]
        (root / manifest["outputs"]["receipt"]).write_bytes(b"corrupt")
        with self.assertRaises(Refusal):
            receive(self.value, self.contract, artifacts, self.fixture.inventory, self.remote.reader)
        self.assertEqual(self.fixture.store.writes, [])

    def test_unknown_nodes_and_missing_results_refuse(self):
        artifacts = self.artifacts()
        changed = dict(artifacts, undeclared=artifacts["produce"])
        with self.assertRaises(Refusal):
            receive(self.value, self.contract, changed, self.fixture.inventory, self.remote.reader)
        self.assertEqual(self.fixture.store.writes, [])
        with self.assertRaises(Refusal):
            receive(self.value, self.contract, {}, self.fixture.inventory, self.remote.reader)

    def test_candidate_provisional_record_is_not_authority(self):
        artifacts = self.artifacts()
        manifest, root = artifacts["produce"]
        changed = copy.deepcopy(manifest)
        changed["key"] = "0" * 64
        artifacts["produce"] = (changed, root)
        with self.assertRaises(Refusal):
            receive(self.value, self.contract, artifacts, self.fixture.inventory, self.remote.reader)
        self.assertEqual(self.fixture.store.writes, [])
