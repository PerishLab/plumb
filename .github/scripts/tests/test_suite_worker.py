import base64
import gzip
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import suite_worker
from lib.blob import Refusal, decode, encode, fingerprint
from lib.suite import graph
import test_suite


class Worker(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        fixture = test_suite.Suite()
        fixture.setUp()
        payload = graph(fixture.value)["nodes"][0]["execution"]["payload"]
        self.value = {"key": "d" * 64, "payload": payload,
                      "inputs": {"contract": {"digest": fingerprint(payload)},
                                 "source": {"digest": "e" * 64},
                                 "configuration": {"digest": payload["configuration"]}},
                      "materialized": {"source": {"key": "e" * 64, "root": str(self.root)}}}
        self.media = encode({"schema": "plumb.source-suite-configuration/v1", "files": {
            "guard-configuration.json": base64.b64encode(b"fixture configuration").decode()}})
        self.calls = []
        self.destination = self.root / "result.json"

    def output(self, command, **options):
        self.calls.append(command)
        if command[:2] == ["rustc", "-vV"]:
            return b"host: x86_64-unknown-linux-gnu\n"
        if command[0] in ("rustc", "cargo") and command[1] == "--version":
            return self.value["payload"]["toolchain"][command[0]].encode()
        if command[:2] == ["cargo", "metadata"]:
            return encode({"packages": [{"name": "plumb-cli", "version": "0.37.6"}]})
        return b"plumb v0.37.6"

    def invoke(self, command, **options):
        self.calls.append(command)
        active = options["env"]
        for secret in ("GH_TOKEN", "PLUMB_PUBLISH_SECRET", "PLUMB_WORKFLOW_INVENTORY_SECRET"):
            self.assertNotIn(secret, active)
        self.assertEqual(active["PLUMB_BUILD_COMMIT"], "a" * 40)
        self.assertEqual(active["PLUMB_BUILD_SOURCE"], "1")
        self.assertNotIn("PLUMB_BUILD_VERSION", active)
        self.assertNotIn("PLUMB_HOME", active)
        if command[:2] == ["cargo", "build"]:
            binary = Path(active["CARGO_TARGET_DIR"]) / "debug/plumb"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"built source suite")
        return subprocess.CompletedProcess(command, 0)

    def execute(self, invoke=None):
        with patch.dict(os.environ, {"PLUMB_WORKFLOW_INVENTORY_URL": "https://inventory.example.test",
                                     "GH_TOKEN": "must-not-inherit", "PLUMB_HOME": "must-not-inherit"}):
            with patch.object(suite_worker.Inventory, "download", return_value=self.media):
                with patch.object(suite_worker.subprocess, "check_output", self.output):
                    with patch.object(suite_worker.subprocess, "run", invoke or self.invoke):
                        suite_worker.execute(self.value, self.destination)

    def test_fixed_suite_builds_tests_and_keeps_true_package_identity(self):
        self.execute()
        result = decode(self.destination.read_bytes())
        receipt = decode(Path(result["evidence"]["receipt"]).read_bytes())
        self.assertEqual(receipt["version"], "v0.37.6")
        self.assertEqual(gzip.decompress(Path(result["outputs"]["binary"]).read_bytes()), b"built source suite")
        self.assertIn(["cargo", "test", "--locked", "--workspace"], self.calls)
        self.assertTrue(any("unittest" in command for command in self.calls))

    def test_failed_tests_emit_no_accepted_result(self):
        def invoke(command, **options):
            if command[:2] == ["cargo", "test"]:
                raise subprocess.CalledProcessError(1, command)
            return self.invoke(command, **options)
        with self.assertRaises(subprocess.CalledProcessError):
            self.execute(invoke)
        self.assertFalse(self.destination.exists())

    def test_changed_configuration_or_payload_refuses_before_execution(self):
        self.value["payload"]["configuration"] = "f" * 64
        with patch.object(suite_worker.subprocess, "run") as run, self.assertRaises(Refusal):
            suite_worker.execute(self.value, self.destination)
        run.assert_not_called()

    def test_configuration_media_cannot_escape_owned_seat(self):
        for name in ("../secret", "/absolute", "C:/secret", "a\\secret"):
            media = decode(self.media)
            media["files"][name] = base64.b64encode(b"bad").decode()
            with tempfile.TemporaryDirectory() as root:
                with self.subTest(name=name), self.assertRaises(Refusal):
                    suite_worker.configuration(encode(media), Path(root) / "configuration")
