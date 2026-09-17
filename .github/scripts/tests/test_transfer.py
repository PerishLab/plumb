import unittest

from lib.blob import Refusal, Unknown, digest
from lib.inventory import Inventory
from support import Memory


class Transfer(unittest.TestCase):
    def setUp(self):
        self.store = Memory()
        self.inventory = Inventory(self.store)

    def test_large_dispatch_has_a_fixed_size_reference(self):
        body = b'{"declaration":"' + b"x" * 100000 + b'"}'
        reference = self.inventory.upload(body)
        self.assertEqual(len(reference), 64)
        self.assertEqual(self.inventory.download(reference), body)
        self.assertTrue(all(key.startswith("v2/blobs/sha256/") for key in self.store.writes))

    def test_unknown_storage_is_not_absence(self):
        def unavailable(key):
            raise Unknown("unavailable")
        self.store.read = unavailable
        with self.assertRaises(Unknown):
            self.inventory.download(digest(b"value"))

    def test_missing_blob_refuses(self):
        with self.assertRaisesRegex(Refusal, "absent"):
            self.inventory.download(digest(b"value"))

    def test_readback_verifies_bytes_even_after_successful_upload(self):
        reference = self.inventory.upload(b"value")
        self.store.objects[self.inventory.blob(reference)] = b"changed"
        with self.assertRaisesRegex(Refusal, "SHA-256"):
            self.inventory.download(reference)

    def test_reference_cannot_select_an_arbitrary_route_or_inline_document(self):
        for reference in ("../secret", "https://other.invalid", "{}", "A" * 64):
            with self.subTest(reference=reference), self.assertRaises(Refusal):
                self.inventory.download(reference)
