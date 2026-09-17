import unittest
from unittest.mock import patch

from lib.blob import Refusal, Unknown, digest
from lib.inventory import Inventory
from lib.readonly import Readonly


class PublicInventory(unittest.TestCase):
    def setUp(self):
        self.store = Readonly("https://inventory.example.test/")
        self.body = b"reusable workload"
        self.value = digest(self.body)
        self.key = "v2/blobs/sha256/" + self.value

    def test_reads_and_verifies_without_credentials(self):
        with patch("lib.readonly.exchange", return_value=(200, {}, self.body)) as exchange:
            self.assertEqual(Inventory(self.store).download(self.value), self.body)
            self.store.verify(self.key, self.value)
        for call in exchange.call_args_list:
            address, method, headers, body = call.args
            self.assertEqual(address, "https://inventory.example.test/" + self.key)
            self.assertEqual(method, "GET")
            self.assertEqual(set(headers), {"host", "cache-control"})
            self.assertEqual(body, b"")

    def test_corrupt_content_refuses(self):
        with patch("lib.readonly.exchange", return_value=(200, {}, b"changed")):
            with self.assertRaises(Refusal):
                self.store.verify(self.key, self.value)
            with self.assertRaises(Refusal):
                Inventory(self.store).download(self.value)

    def test_public_failure_never_proves_absence(self):
        for status in (301, 302, 403, 404, 429, 500, 503):
            with self.subTest(status=status), patch("lib.readonly.exchange", return_value=(status, {}, b"")):
                with self.assertRaises(Unknown):
                    self.store.read(self.key)
        with patch("lib.readonly.exchange", side_effect=TimeoutError("private address")):
            with self.assertRaises(Unknown) as caught:
                self.store.read(self.key)
            self.assertNotIn("private address", str(caught.exception))

    def test_write_is_refused_before_transport(self):
        with patch("lib.readonly.exchange") as exchange:
            with self.assertRaises(Refusal):
                Inventory(self.store).upload(self.body)
            exchange.assert_not_called()

    def test_origin_and_key_are_closed(self):
        for origin in ("http://example.test", "https://user:secret@example.test",
                       "https://example.test/path", "https://example.test?key=secret",
                       "https://example.test/#route", "https://example.test\n", None):
            with self.subTest(origin=origin), self.assertRaises(Refusal):
                Readonly(origin)
        for key in ("../secret", self.key + "?token=value", self.key + "/extra", None):
            with self.subTest(key=key), self.assertRaises(Refusal):
                self.store.read(key)
