import base64
import http.client
import unittest
from unittest.mock import patch

from lib.blob import Refusal, Unknown, digest
from lib.r2 import R2


class Storage(unittest.TestCase):
    def setUp(self):
        self.store = R2("https://example.r2.cloudflarestorage.com", "inventory",
                        {"access": "fixture", "secret": "fixture"})
        self.key = "v2/blobs/sha256/" + digest(b"body")

    def failure(self, code, status=403):
        return status, {}, ("<Error><Code>" + code + "</Code></Error>").encode()

    def test_only_explicit_missing_key_is_a_miss(self):
        with patch("lib.r2.exchange", return_value=self.failure("NoSuchKey", 404)):
            self.assertIsNone(self.store.read(self.key))
        for code in ("AccessDenied", "NoSuchBucket", "500", "404", "ExpiredToken"):
            with self.subTest(code=code):
                with patch("lib.r2.exchange", return_value=self.failure(code, 404)):
                    with self.assertRaises(Unknown):
                        self.store.read(self.key)
        with patch("lib.r2.exchange", return_value=self.failure("NoSuchKey", 500)):
            with self.assertRaises(Unknown):
                self.store.read(self.key)

    def test_uncertain_write_is_not_success(self):
        for error in (TimeoutError(), OSError(), http.client.IncompleteRead(b"partial")):
            with self.subTest(error=type(error).__name__):
                with patch("lib.r2.exchange", side_effect=error):
                    with self.assertRaisesRegex(Unknown, "put-object transport unavailable"):
                        self.store.create(self.key, b"body")

    def test_safe_operation_diagnostics_do_not_expose_raw_transport_output(self):
        cases = [(self.failure("AccessDenied"), "AccessDenied"),
                 (self.failure("private credential detail"), "HTTP 403"),
                 ((500, {}, b"private credential detail"), "HTTP 500")]
        for result, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                with patch("lib.r2.exchange", return_value=result):
                    with self.assertRaises(Unknown) as raised:
                        self.store.read(self.key)
                self.assertEqual(str(raised.exception),
                                 f"inventory get-object failed ({diagnostic}); existence is unknown")

    def test_conditional_conflict_requires_readback(self):
        with patch("lib.r2.exchange", return_value=self.failure("PreconditionFailed", 412)):
            self.assertIsNone(self.store.create(self.key, b"body"))
        with patch("lib.r2.exchange", return_value=self.failure("PreconditionFailed", 500)):
            with self.assertRaises(Unknown):
                self.store.create(self.key, b"body")

    def test_upload_is_conditional_signed_and_checksum_verified(self):
        with patch("lib.r2.exchange", return_value=(200, {}, b"")) as call:
            self.store.create(self.key, b"body")
            address, method, headers, body = call.call_args.args
            self.assertEqual(method, "PUT")
            self.assertEqual(address, self.store.endpoint + "/inventory/" + self.key)
            self.assertEqual(body, b"body")
            self.assertEqual(headers["if-none-match"], "*")
            self.assertEqual(headers["content-length"], "4")
            self.assertEqual(base64.b64decode(headers["x-amz-checksum-sha256"]).hex(), digest(body))
            self.assertIn("if-none-match", headers["authorization"])
            self.assertIn("x-amz-checksum-sha256", headers["authorization"])

    def test_plan_checks_head_not_payload(self):
        expected = digest(b"body")
        checksum = base64.b64encode(bytes.fromhex(expected)).decode()
        with patch("lib.r2.exchange", return_value=(200, {"X-Amz-Checksum-Sha256": checksum}, b"")) as call:
            self.store.verify(self.key, expected)
            self.assertEqual(call.call_args.args[1], "HEAD")
            self.assertEqual(call.call_args.args[2]["x-amz-checksum-mode"], "ENABLED")
        with patch("lib.r2.exchange", return_value=(404, {}, b"")):
            with self.assertRaises(Unknown):
                self.store.verify(self.key, expected)

    def test_metadata_or_etag_is_not_a_sha256_proof(self):
        headers = {"ETag": digest(b"body"), "x-amz-meta-sha256": digest(b"body")}
        with patch("lib.r2.exchange", return_value=(200, headers, b"")):
            with self.assertRaises(Refusal):
                self.store.verify(self.key, digest(b"body"))

    def test_protocol_cannot_overwrite_legacy_routes(self):
        with patch("lib.r2.exchange") as call:
            with self.assertRaises(Refusal):
                self.store.create("v1/inventory.json", b"body")
            with self.assertRaises(Refusal):
                self.store.call("DELETE", self.key)
            call.assert_not_called()

    def test_redirect_is_unknown_not_an_authority_change(self):
        with patch("lib.r2.exchange", return_value=(307, {"Location": "https://other.test"}, b"")) as call:
            with self.assertRaises(Unknown):
                self.store.read(self.key)
            call.assert_called_once()

    def test_environment_uses_only_explicit_inventory_authority(self):
        values = {"PLUMB_WORKFLOW_INVENTORY_" + key: value for key, value in
                  {"ACCESS": "fixture", "SECRET": "fixture-secret", "BUCKET": "inventory",
                   "ENDPOINT": self.store.endpoint}.items()}
        values.update(AWS_ACCESS_KEY_ID="ambient", AWS_SESSION_TOKEN="ambient")
        with patch.dict("os.environ", values, clear=True):
            self.assertEqual(R2.environment().identity,
                             {"access": "fixture", "secret": "fixture-secret", "region": "auto"})
        with patch.dict("os.environ", {"AWS_ACCESS_KEY_ID": "ambient"}, clear=True):
            with self.assertRaises(Refusal):
                R2.environment()


if __name__ == "__main__":
    unittest.main()
