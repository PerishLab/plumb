import os
import tempfile
import tomllib
import unittest
from pathlib import Path

from lib.blob import Refusal, digest
from lib.document import project
from lib.snapshot import Snapshot


class Materialization(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.destination = Path(self.directory.name) / "source"

    def snapshot(self, files=None):
        return Snapshot({"paths": ["code"]}, files or {"code/main": ("100755", b"source")})

    def test_materialized_bytes_are_the_hashed_bytes(self):
        snapshot = self.snapshot()
        self.assertEqual(snapshot.materialize(self.destination), snapshot.key)
        body = (self.destination / "code/main").read_bytes()
        self.assertEqual(digest(body), snapshot.manifest["references"][0]["digest"])

    def test_changed_missing_and_extra_files_refuse(self):
        snapshot = self.snapshot()
        snapshot.materialize(self.destination)
        path = self.destination / "code/main"
        path.write_bytes(b"different")
        with self.assertRaises(Refusal):
            snapshot.verify(self.destination)
        path.unlink()
        with self.assertRaises(Refusal):
            snapshot.verify(self.destination)
        path.write_bytes(b"source")
        path.chmod(0o755)
        (self.destination / "extra").write_bytes(b"unexpected")
        with self.assertRaises(Refusal):
            snapshot.verify(self.destination)

    def test_existing_destination_is_never_overwritten(self):
        self.destination.mkdir()
        with self.assertRaises(Refusal):
            self.snapshot().materialize(self.destination)

    def test_unsafe_paths_refuse_before_creation(self):
        for name in ["../outside", "/absolute", "a/../b", ".git/config", "C:/file", "a\\b"]:
            with self.subTest(name=name), self.assertRaises(Refusal):
                self.snapshot({name: ("100644", b"x")}).materialize(self.destination)
            self.assertFalse(self.destination.exists())

    def test_file_cannot_also_be_a_parent(self):
        with self.assertRaises(Refusal):
            self.snapshot({"a": ("120000", b"."), "a/b": ("100644", b"x")}).materialize(self.destination)
        self.assertFalse(self.destination.exists())

    @unittest.skipIf(os.name == "nt", "symlink creation requires Windows runner privileges")
    def test_symlink_preserves_exact_target(self):
        snapshot = self.snapshot({"a": ("100644", b"x"), "b": ("120000", b"a")})
        snapshot.materialize(self.destination)
        self.assertEqual(os.readlink(self.destination / "b"), "a")
        snapshot.verify(self.destination)

    def test_external_symlink_refuses(self):
        with self.assertRaises(Refusal):
            self.snapshot({"a": ("120000", b"../outside")}).materialize(self.destination)
        self.assertFalse(self.destination.exists())


class Documents(unittest.TestCase):
    def test_toml_projection_remains_toml_and_preserves_other_values(self):
        original = b'[package]\nname="fixture"\nversion="1.2.3"\n[dependencies]\nexternal="9"\n'
        body = project(original, {"format": "toml", "set": {"/package/version": "0.0.0"}})
        value = tomllib.loads(body.decode())
        self.assertEqual(value, {"package": {"name": "fixture", "version": "0.0.0"},
                                 "dependencies": {"external": "9"}})

    def test_nested_arrays_and_unicode_roundtrip(self):
        original = '[[package]]\nname="🦀"\nversion="1"\ndependencies=["a", "b"]\n'.encode()
        body = project(original, {"format": "toml", "set": {"/package/0/version": "0"}})
        self.assertEqual(tomllib.loads(body.decode())["package"][0]["name"], "🦀")

    def test_hash_only_omit_contract_is_retired(self):
        with self.assertRaises(Refusal):
            project(b'{"version":"1"}', {"format": "json", "omit": ["/version"]})

    def test_ambiguous_overlapping_projection_refuses(self):
        with self.assertRaises(Refusal):
            project(b'{"package":{"version":"1"}}',
                    {"format": "json", "set": {"/package": {}, "/package/version": "0"}})

    def test_unrepresentable_toml_value_refuses(self):
        with self.assertRaises(Refusal):
            project(b'version="1"', {"format": "toml", "set": {"/version": None}})
