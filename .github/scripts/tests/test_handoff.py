import copy
import tempfile
import unittest
from pathlib import Path

from lib.blob import Refusal
from lib.declaration import Declaration
from lib.handoff import inspect, pack, record
from support import Fixture


class Handoff(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.fixture = Fixture()
        self.body = self.root / "candidate"
        self.body.write_bytes(b"unprivileged workload")
        self.key = self.fixture.plan()["nodes"]["produce"]["key"]
        self.result = {"key": self.key, "outputs": {"content": str(self.body)}}
        self.seat = self.root / "handoff"

    def test_unprivileged_pack_does_not_write_shared_inventory(self):
        manifest = pack(self.result, self.seat)
        self.assertEqual(self.fixture.store.writes, [])
        self.assertNotIn(str(self.body), str(manifest))
        receipt = record(Declaration(self.fixture.value), "produce", manifest,
                         self.seat, self.fixture.inventory)
        self.assertEqual(receipt["key"], self.key)
        self.assertEqual(self.fixture.plan()["nodes"]["produce"]["state"], "reuse")

    def test_producer_key_cannot_override_trusted_plan(self):
        manifest = pack(self.result, self.seat)
        manifest["key"] = "0" * 64
        with self.assertRaises(Refusal):
            record(Declaration(self.fixture.value), "produce", manifest, self.seat, self.fixture.inventory)
        self.assertEqual(self.fixture.store.writes, [])

    def test_external_path_cannot_be_sent_to_privileged_collector(self):
        manifest = pack(self.result, self.seat)
        for value in ("../../secret", "/etc/passwd", "C:\\secret", "a" * 63):
            changed = copy.deepcopy(manifest)
            changed["outputs"]["content"] = value
            with self.subTest(value=value), self.assertRaises(Refusal):
                inspect(changed, self.seat)

    def test_modified_blob_refuses_before_shared_writes(self):
        manifest = pack(self.result, self.seat)
        (self.seat / manifest["outputs"]["content"]).write_bytes(b"modified")
        with self.assertRaises(Refusal):
            record(Declaration(self.fixture.value), "produce", manifest, self.seat, self.fixture.inventory)
        self.assertEqual(self.fixture.store.writes, [])

    def test_symlink_cannot_exfiltrate_collector_files(self):
        manifest = pack(self.result, self.seat)
        blob = self.seat / manifest["outputs"]["content"]
        blob.unlink()
        try:
            blob.symlink_to(self.body)
        except OSError:
            self.skipTest("host does not permit fixture symlinks")
        with self.assertRaises(Refusal):
            inspect(manifest, self.seat)

    def test_additional_outputs_and_evidence_are_not_trusted(self):
        manifest = pack(self.result, self.seat)
        for field in ("outputs", "evidence"):
            changed = copy.deepcopy(manifest)
            changed[field]["forged"] = manifest["outputs"]["content"]
            with self.subTest(field=field), self.assertRaises(Refusal):
                record(Declaration(self.fixture.value), "produce", changed,
                       self.seat, self.fixture.inventory)
        self.assertEqual(self.fixture.store.writes, [])
