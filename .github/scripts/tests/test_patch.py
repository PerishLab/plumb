import unittest

from lib.blob import Refusal, digest
from lib.patch import apply
from lib.source import Source
import test_source


class Patches(unittest.TestCase):
    def patch(self, source=b"abc", result=b"aZc", edits=None):
        return {"source": digest(source), "result": digest(result),
                "edits": [[1, 2, "Z"]] if edits is None else edits}

    def test_binary_bytes_need_no_format_parser(self):
        self.assertEqual(apply(b"\xffbc", self.patch(b"\xffbc", b"\xffZc")), b"\xffZc")

    def test_input_and_output_digests_are_verified(self):
        for field in ("source", "result"):
            patch = self.patch()
            patch[field] = "0" * 64
            with self.assertRaises(Refusal):
                apply(b"abc", patch)

    def test_ambiguous_ranges_refuse(self):
        for edits in ([[1, 3, "Z"], [2, 3, "x"]], [[-1, 2, "Z"]],
                      [[1, 4, "Z"]], [[True, 2, "Z"]], [[2, 1, "Z"]]):
            with self.subTest(edits=edits), self.assertRaises(Refusal):
                apply(b"abc", self.patch(edits=edits))


class PreparedSource(unittest.TestCase):
    setUp = test_source.GitSource.setUp
    git = test_source.GitSource.git
    commit = test_source.GitSource.commit

    def prepared(self, version):
        body = f'[package]\nversion="{version}"\nexternal="9"\n'.encode()
        result = b'[package]\nversion="0.0.0"\nexternal="9"\n'
        (self.root / "Cargo.toml").write_bytes(body)
        self.commit()
        start = body.index(b'"')
        return {"paths": ["Cargo.toml"], "projects": {
            "Cargo.toml": {"format": "toml", "set": {"/package/version": "0.0.0"}}},
            "patches": {"Cargo.toml": {
                "source": digest(body), "result": digest(result),
                "edits": [[start, start + len(version) + 2, '"0.0.0"']]}}}

    def test_transport_identity_and_offsets_are_not_content_keys(self):
        first = Source(self.root).snapshot(self.prepared("1"))
        second = Source(self.root).snapshot(self.prepared("20.30.400-beta.5"))
        self.assertEqual(first.key, second.key)
        second.materialize(self.root / "prepared")
        self.assertEqual((self.root / "prepared/Cargo.toml").read_bytes(),
                         b'[package]\nversion="0.0.0"\nexternal="9"\n')

    def test_stale_preparation_cannot_hide_a_source_change(self):
        recipe = self.prepared("1")
        (self.root / "Cargo.toml").write_text('[package]\nversion="1"\nexternal="10"\n')
        self.commit()
        with self.assertRaises(Refusal):
            Source(self.root).snapshot(recipe)
