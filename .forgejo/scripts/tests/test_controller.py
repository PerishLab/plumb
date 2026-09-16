import io
import gzip
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from urllib.error import HTTPError, URLError

import controller
import ship
from lib.blob import Refusal, Unknown, decode, digest, fingerprint
from lib.material import download


class Controller(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.contract = {"schema": "plumb.controller-build/v1", "format": "gzip-6", "version": "v1.0.0",
                         "channel": "stable", "commit": "a" * 40, "target": "x86_64-unknown-linux-gnu",
                         "rustc": "rustc fixture", "cargo": "cargo fixture"}
        self.contract["environment"] = {"inherit": ["PATH"], "managed": [], "reject": []}
        self.request = {"key": "b" * 64, "payload": self.contract,
                        "inputs": {"contract": {"digest": fingerprint(self.contract)},
                                   "source": {"digest": "c" * 64}},
                        "materialized": {"source": {"key": "c" * 64, "root": str(self.root)}}}
        self.result = self.root / "result.json"
        probe = patch.object(controller.subprocess, "check_output", side_effect=lambda argv, **kwargs: (argv[0] + " fixture").encode())
        probe.start()
        self.addCleanup(probe.stop)

    def build(self, command, **options):
        self.assertEqual(command[:4], ["cargo", "build", "--locked", "--bin"])
        self.assertEqual(options["env"]["PLUMB_BUILD_COMMIT"], "a" * 40)
        target = Path(options["env"]["CARGO_TARGET_DIR"])
        binary = target / self.contract["target"] / "debug/plumb"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"controller executable")
        return subprocess.CompletedProcess(command, 0)

    def test_controller_build_returns_compressed_blob_and_proof(self):
        with patch.object(controller.subprocess, "run", self.build):
            controller.execute(self.request, self.result)
        result = decode(self.result.read_bytes())
        self.assertEqual(gzip.decompress(Path(result["outputs"]["content"]).read_bytes()), b"controller executable")
        self.assertEqual(decode(Path(result["evidence"]["receipt"]).read_bytes())["contract"], self.contract)

    def test_changed_transport_identity_cannot_escape_its_input_digest(self):
        self.contract["version"] = "v2.0.0"
        with patch.object(controller.subprocess, "run") as run, self.assertRaises(Refusal):
            controller.execute(self.request, self.result)
        run.assert_not_called()

    def test_compression_does_not_capture_seat_names_or_timestamps(self):
        with patch.object(controller.subprocess, "run", self.build):
            controller.execute(self.request, self.result)
            first = Path(decode(self.result.read_bytes())["outputs"]["content"]).read_bytes()
            controller.execute(self.request, self.result)
            second = Path(decode(self.result.read_bytes())["outputs"]["content"]).read_bytes()
        self.assertEqual(first, second)

    def test_undeclared_controller_format_is_not_guessed(self):
        self.contract["format"] = "plain"
        self.request["inputs"]["contract"]["digest"] = fingerprint(self.contract)
        with patch.object(controller.subprocess, "run") as run, self.assertRaises(Refusal):
            controller.execute(self.request, self.result)
        run.assert_not_called()

    def test_build_failure_never_returns_success(self):
        with patch.object(controller.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "cargo")):
            with self.assertRaises(subprocess.CalledProcessError):
                controller.execute(self.request, self.result)
        self.assertFalse(self.result.exists())

    def test_cached_controller_still_installs_into_actual_runner_seat(self):
        request = {"preparation": [{"node": "controller", "output": "content"}],
                   "payload": {"controller": {"marker": {"name": "v1.0.0", "sha256": "a" * 64}, "generation": "b" * 64}},
                   "producers": {"controller": {"outputs": {"content": {"digest": "d" * 64}}}}}

        def materialize(reference, path):
            path.write_bytes(gzip.compress(b"controller", mtime=0))
            return path

        environment = {}
        with patch.object(ship, "download", materialize), patch.object(ship.subprocess, "run") as run:
            with patch.dict(ship.os.environ, {"PLUMB_BUILD_CONFIGURATION": "v99.0.0"}):
                tool = ship.controller(request, self.root, environment)
        self.assertTrue(Path(tool).is_file())
        self.assertEqual(environment["PLUMB_HOME"], str(self.root / "home"))
        self.assertEqual(run.call_args.args[0][1:], ["configuration", "install", str(Path(ship.__file__).resolve().parents[2]),
                         "--marker", "v1.0.0", "--generation", "b" * 64, "--path", str(self.root / "home/configurations")])

    def test_missing_binding_refuses_before_download_or_install(self):
        with patch.object(ship, "download") as download, patch.object(ship.subprocess, "run") as run:
            with self.assertRaises(Refusal):
                ship.controller({}, self.root, {})
        download.assert_not_called()
        run.assert_not_called()

    def test_bad_generation_refuses_before_download_or_install(self):
        request = {"payload": {"controller": {"marker": {"name": "v1.0.0", "sha256": "a" * 64}, "generation": "latest"}}}
        with patch.object(ship, "download") as download, patch.object(ship.subprocess, "run") as run:
            with self.assertRaises(Refusal):
                ship.controller(request, self.root, {})
        download.assert_not_called()
        run.assert_not_called()


class Material(unittest.TestCase):
    def test_verified_content_is_materialized_without_archive_interpretation(self):
        body = b"opaque bytes"
        url = "https://blobs.example/v2/blobs/sha256/" + digest(body)
        response = io.BytesIO(body)
        response.url = url
        reference = {"digest": digest(body), "reuse": {"type": "workload", "source": url}}
        with tempfile.TemporaryDirectory() as root, patch("lib.material.urlopen", return_value=response) as opened:
            path = download(reference, Path(root) / "binary")
            self.assertEqual(path.read_bytes(), body)
            request = opened.call_args.args[0]
            self.assertEqual(request.full_url, url)
            self.assertEqual(request.get_header("User-agent"), "plumb-workflow/1")
            self.assertEqual(opened.call_args.kwargs, {"timeout": 30})

    def test_transport_failures_remain_unknown_without_materializing_content(self):
        url = "https://blobs.example/v2/blobs/sha256/" + "a" * 64
        reference = {"digest": "a" * 64, "reuse": {"type": "workload", "source": url}}
        failures = [(HTTPError(url, 403, "private response", {}, None), "HTTP 403"),
                    (URLError("private transport detail"), "URLError"),
                    (TimeoutError("private transport detail"), "TimeoutError")]
        for error, diagnostic in failures:
            with self.subTest(diagnostic=diagnostic), tempfile.TemporaryDirectory() as root:
                path = Path(root) / "binary"
                with patch("lib.material.urlopen", side_effect=error):
                    with self.assertRaises(Unknown) as raised:
                        download(reference, path)
                self.assertEqual(str(raised.exception), "workload download unavailable: " + diagnostic)
                self.assertIs(raised.exception.__cause__, error)
                self.assertFalse(path.exists())

    def test_corruption_never_creates_executable(self):
        url = "https://blobs.example/v2/blobs/sha256/" + "a" * 64
        response = io.BytesIO(b"corrupt")
        response.url = url
        reference = {"digest": "a" * 64, "reuse": {"type": "workload", "source": url}}
        with tempfile.TemporaryDirectory() as root, patch("lib.material.urlopen", return_value=response):
            path = Path(root) / "binary"
            with self.assertRaises(Refusal):
                download(reference, path)
            self.assertFalse(path.exists())
