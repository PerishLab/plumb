import io
import os
import shutil
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import runtime
from lib.blob import Refusal, decode, fingerprint
from lib.bundle import checksum, unpack
from lib.runtime import activate


class Runtime(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.archive = self.root / "source.tar"
        self.package("distribution/tool/bin/compiler", b"compiler")
        self.contract = {
            "schema": "plumb.runtime-bundle/v1",
            "files": [],
            "archives": [{"url": "https://source.example/tool.tar", "sha256": checksum(self.archive),
                          "prefix": "distribution/tool"}],
            "probes": [{"path": "bin/compiler", "stdout": "compiler 1"}],
            "environment": {"CARGO_HOME": "cargo", "RUSTUP_HOME": "rustup", "RUSTUP_TOOLCHAIN": "tools"},
            "path": ["tools/bin"]}
        self.request = {"key": "a" * 64, "payload": self.contract,
                        "inputs": {"contract": {"digest": fingerprint(self.contract)}}}

    def package(self, name, content):
        with tarfile.open(self.archive, "w") as archive:
            member = tarfile.TarInfo(name)
            member.size = len(content)
            member.mode = 0o755
            archive.addfile(member, io.BytesIO(content))

    def produce(self):
        result = self.root / "result.json"
        with patch.object(runtime, "fetch", side_effect=lambda item, path: shutil.copyfile(self.archive, path)):
            runtime.execute(self.request, result)
        return Path(decode(result.read_bytes())["outputs"]["content"])

    def consume(self, archive):
        request = {"preparation": [{"node": "runtime/test", "output": "content"}],
                   "inputs": {"runtime": {"digest": fingerprint(self.contract)}},
                   "producers": {"runtime/test": {"outputs": {"content": {"digest": checksum(archive)}}}}}
        def download(reference, path):
            shutil.copyfile(archive, path)
            return path
        with patch("lib.runtime.download", download), patch("lib.runtime.subprocess.check_output", return_value=b"compiler 1"):
            return activate(request, self.root, {"PATH": "original"})

    def test_bundle_is_reusable_but_activation_is_fresh_and_job_local(self):
        before = dict(os.environ)
        first = self.produce()
        second = self.produce()
        self.assertEqual(first.read_bytes(), second.read_bytes())
        active = self.consume(first)
        again = self.consume(first)
        self.assertNotEqual(active["PATH"], again["PATH"])
        self.assertTrue(active["PATH"].endswith(os.pathsep + "original"))
        self.assertTrue(Path(active["RUSTUP_TOOLCHAIN"]).is_relative_to(self.root))
        self.assertEqual(dict(os.environ), before)

    def test_no_runtime_requirement_does_not_download_or_probe(self):
        with patch("lib.runtime.download") as download:
            self.assertEqual(activate({}, self.root, {}), {})
        download.assert_not_called()

    def test_changed_contract_refuses_before_source_fetch(self):
        self.contract["probes"][0]["stdout"] = "compiler 2"
        with patch.object(runtime, "fetch") as fetch, self.assertRaises(Refusal):
            runtime.execute(self.request, self.root / "result")
        fetch.assert_not_called()

    def test_source_checksum_is_required_before_execution(self):
        with patch.object(runtime, "exchange", return_value=(200, {}, b"corrupt")):
            with self.assertRaisesRegex(Refusal, "checksum"):
                runtime.execute(self.request, self.root / "result")
        self.assertFalse((self.root / "result").exists())

    def test_prepared_archive_checksum_is_rechecked(self):
        self.contract["archives"][0]["sha256"] = "a" * 64
        self.request["inputs"]["contract"]["digest"] = fingerprint(self.contract)
        archive = self.produce()
        with self.assertRaisesRegex(Refusal, "checksum"):
            self.consume(archive)

    def test_actual_tool_version_must_equal_the_locked_world(self):
        self.contract["probes"][0]["stdout"] = "compiler 2"
        self.request["inputs"]["contract"]["digest"] = fingerprint(self.contract)
        with self.assertRaisesRegex(Refusal, "expected 'compiler 2', got 'compiler 1'"):
            self.consume(self.produce())

    def test_independent_executable_uses_the_same_verified_bundle(self):
        self.contract["files"] = [{"url": "https://source.example/helper",
                                  "sha256": checksum(self.archive), "path": "bin/helper",
                                  "executable": True}]
        self.request["inputs"]["contract"]["digest"] = fingerprint(self.contract)
        active = self.consume(self.produce())
        helper = Path(active["RUSTUP_TOOLCHAIN"]) / "bin/helper"
        self.assertEqual(helper.read_bytes(), self.archive.read_bytes())
        if os.name != "nt":
            self.assertTrue(helper.stat().st_mode & 0o111)

    def test_independent_file_cannot_replace_an_archive_member(self):
        self.contract["files"] = [{"url": "https://source.example/helper",
                                  "sha256": checksum(self.archive), "path": "bin/compiler",
                                  "executable": True}]
        self.request["inputs"]["contract"]["digest"] = fingerprint(self.contract)
        with self.assertRaisesRegex(Refusal, "overlaps"):
            self.consume(self.produce())

    def test_archive_cannot_escape_or_overwrite(self):
        destination = self.root / "tools"
        destination.mkdir()
        for name in ("distribution/tool/../outside", "distribution/tool/C:/outside"):
            self.package(name, b"bad")
            with self.assertRaises(Refusal):
                unpack(self.archive, "distribution/tool", destination)
        self.package("distribution/tool/bin/compiler", b"good")
        unpack(self.archive, "distribution/tool", destination)
        with self.assertRaises(Refusal):
            unpack(self.archive, "distribution/tool", destination)

    def test_probe_paths_and_source_authority_are_bounded(self):
        for field, value in (("url", "http://source.example/tool"), ("prefix", "../tool")):
            self.contract["archives"][0][field] = value
            with self.assertRaises(Refusal):
                runtime.contract(self.contract)
