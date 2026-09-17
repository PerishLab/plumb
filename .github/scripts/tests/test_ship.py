import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import ship
from lib.blob import Refusal, decode, encode


class Bridge(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "request.json"
        self.destination = self.root / "candidate.json"
        self.source.write_bytes(encode({"schema": "plumb.blob-execution/v1", "key": "a" * 64,
                                       "node": "ship/fixture", "payload": {"opaque": True}}))
        self.status = 0
        self.action = "ship/fixture"
        self.calls = []
        self.controller = patch.object(ship, "controller", return_value="plumb")
        self.controller.start()
        self.addCleanup(self.controller.stop)

    def invoke(self, command, **options):
        self.calls.append((command, options))
        self.assertEqual(command[:3], ["plumb", "ship", "execute"])
        output = Path(command[command.index("--output") + 1])
        self.assertTrue(output.is_relative_to(self.root), "fixture output escaped its temporary root")
        content = output.parent / "content"
        content.write_bytes(b"opaque workload")
        output.write_bytes(encode({"schema": "plumb.ship-result/v2", "marker": "b" * 64,
                                  "action": self.action, "evidence": {"business": "proof"}, "projection": {
                                      "workload": str(content), "publication": "",
                                      "receipt": {"business": "proof"}}}))
        return subprocess.CompletedProcess(command, self.status)

    def test_bridge_transports_context_and_separates_content_from_evidence(self):
        with patch.object(ship.subprocess, "run", self.invoke):
            ship.execute(self.source, self.destination)
        result = decode(self.destination.read_bytes())
        self.assertEqual(result["key"], "a" * 64)
        self.assertEqual(Path(result["outputs"]["content"]).read_bytes(), b"opaque workload")
        self.assertEqual(decode(Path(result["evidence"]["receipt"]).read_bytes()), {"business": "proof"})

    def test_failure_does_not_publish_a_candidate_even_if_result_exists(self):
        self.status = 3
        with patch.object(ship.subprocess, "run", self.invoke), self.assertRaises(Refusal):
            ship.execute(self.source, self.destination)
        self.assertFalse(self.destination.exists())

    def test_wrong_action_refuses(self):
        self.action = "ship/different"
        with patch.object(ship.subprocess, "run", self.invoke), self.assertRaises(Refusal):
            ship.execute(self.source, self.destination)
        self.assertFalse(self.destination.exists())

    def test_retry_gets_an_independent_artifact_seat(self):
        with patch.object(ship.subprocess, "run", self.invoke):
            ship.execute(self.source, self.destination)
            ship.execute(self.source, self.destination)
        first, second = [call[1]["env"]["PLUMB_RELEASE_ARTIFACTS"] for call in self.calls]
        self.assertNotEqual(first, second)
