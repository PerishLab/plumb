import base64
import subprocess
import unittest
from unittest.mock import patch

from lib.blob import Refusal, Unknown, digest, encode
from lib.r2 import R2


class Storage(unittest.TestCase):
    def setUp(self):
        self.store = R2("https://example.r2.cloudflarestorage.com", "inventory", {})
        self.key = "v2/blobs/sha256/" + digest(b"body")

    def result(self, code=0, stdout=b"{}", stderr=b""):
        return subprocess.CompletedProcess([], code, stdout, stderr)

    def failure(self, code):
        return self.result(1, stderr=("An error occurred (" + code +
                                     ") when calling the GetObject operation").encode())

    def test_only_explicit_missing_key_is_a_miss(self):
        with patch("lib.r2.subprocess.run", return_value=self.failure("NoSuchKey")):
            self.assertIsNone(self.store.read(self.key))
        for code in ("AccessDenied", "NoSuchBucket", "500", "404", "ExpiredToken"):
            with self.subTest(code=code):
                with patch("lib.r2.subprocess.run", return_value=self.failure(code)):
                    with self.assertRaises(Unknown):
                        self.store.read(self.key)

    def test_uncertain_write_is_not_success(self):
        with patch("lib.r2.subprocess.run", side_effect=subprocess.TimeoutExpired("aws", 60)):
            with self.assertRaisesRegex(Unknown, "put-object transport unavailable: TimeoutExpired"):
                self.store.create(self.key, b"body")

    def test_safe_operation_diagnostics_do_not_expose_raw_transport_output(self):
        cases = [(self.failure("AccessDenied"), "AccessDenied"),
                 (self.failure("404"), "404"),
                 (self.failure("private credential detail"), "exit 1"),
                 (self.result(7, stderr=b"private credential detail"), "exit 7")]
        for result, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                with patch("lib.r2.subprocess.run", return_value=result):
                    with self.assertRaises(Unknown) as raised:
                        self.store.read(self.key)
                self.assertEqual(str(raised.exception),
                                 f"inventory get-object failed ({diagnostic}); existence is unknown")

    def test_conditional_conflict_requires_readback(self):
        with patch("lib.r2.subprocess.run", return_value=self.failure("PreconditionFailed")):
            self.assertIsNone(self.store.create(self.key, b"body"))

    def test_upload_is_conditional_and_checksum_verified(self):
        with patch("lib.r2.subprocess.run", return_value=self.result()) as run:
            self.store.create(self.key, b"body")
            command = run.call_args.args[0]
            self.assertEqual(command[command.index("--if-none-match") + 1], "*")
            checksum = command[command.index("--checksum-sha256") + 1]
            self.assertEqual(base64.b64decode(checksum).hex(), digest(b"body"))

    def test_plan_checks_head_not_payload(self):
        expected = digest(b"body")
        checksum = base64.b64encode(bytes.fromhex(expected)).decode()
        response = self.result(stdout=encode({"ChecksumSHA256": checksum}))
        with patch("lib.r2.subprocess.run", return_value=response) as run:
            self.store.verify(self.key, expected)
            self.assertIn("head-object", run.call_args.args[0])
            self.assertNotIn("get-object", run.call_args.args[0])

    def test_metadata_or_etag_is_not_a_sha256_proof(self):
        response = self.result(stdout=encode({"ETag": digest(b"body"),
                                              "Metadata": {"sha256": digest(b"body")}}))
        with patch("lib.r2.subprocess.run", return_value=response):
            with self.assertRaises(Refusal):
                self.store.verify(self.key, digest(b"body"))

    def test_protocol_cannot_overwrite_legacy_routes(self):
        with patch("lib.r2.subprocess.run") as run:
            with self.assertRaises(Refusal):
                self.store.create("v1/inventory.json", b"body")
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
