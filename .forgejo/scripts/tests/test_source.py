import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from lib.blob import Refusal, fingerprint
from lib.source import Source, Sources, project, resolve
from support import declaration


class Projection(unittest.TestCase):
    def test_explicit_literal_reference_binds_effective_values(self):
        value = declaration()
        contract = {"target": "linux", "version": "controller-A"}
        value["nodes"][0]["inputs"]["contract"] = {"value": contract}
        held = resolve(value, None)
        self.assertEqual(held["nodes"][0]["inputs"]["contract"], fingerprint(contract))
        contract["version"] = "controller-B"
        self.assertNotEqual(held["nodes"][0]["inputs"]["contract"],
                            resolve(value, None)["nodes"][0]["inputs"]["contract"])

    def test_json_version_only_changes_have_equal_content(self):
        recipe = {"format": "json", "set": {"/version": "0.0.0"}}
        self.assertEqual(project(b'{"version":"1","code":"same"}', recipe),
                         project(b'{"version":"2","code":"same"}', recipe))

    def test_toml_is_not_interpreted_by_the_blob_control_plane(self):
        recipe = {"format": "toml", "set": {"/workspace/package/version": "0.0.0"}}
        with self.assertRaises(Refusal):
            project(b'[workspace.package]\nversion="1"\nlicense="MIT"\n', recipe)

    def test_actual_content_change_invalidates(self):
        recipe = {"format": "json", "set": {"/version": "0.0.0"}}
        self.assertNotEqual(project(b'{"version":"1","code":"before"}', recipe),
                            project(b'{"version":"1","code":"after"}', recipe))

    def test_missing_pointer_is_not_silently_ignored(self):
        with self.assertRaises(Refusal):
            project(b'{"code":"same"}', {"format": "json", "set": {"/version": "0.0.0"}})

    def test_array_pointer_and_escape(self):
        recipe = {"format": "json", "set": {"/packages/0/a~1b": None}}
        held = project(b'{"packages":[{"a/b":"1"}]}', recipe)
        self.assertEqual(json.loads(held), {"packages": [{"a/b": None}]})

    def test_pointer_cannot_traverse_or_replace_a_scalar(self):
        for body, pointer in [(b'"version"', "/0"), (b'{"version":"1"}', "/version/0"),
                              (b'{"version":null}', "/version/x")]:
            with self.subTest(body=body, pointer=pointer), self.assertRaises(Refusal):
                project(body, {"format": "json", "set": {pointer: None}})

    def test_array_indices_require_canonical_ascii(self):
        for pointer in ["/01", "/-1", "/1", "/٠", "/-"]:
            with self.subTest(pointer=pointer), self.assertRaises(Refusal):
                project(b'["version"]', {"format": "json", "set": {pointer: None}})


class GitSource(unittest.TestCase):
    def test_named_sources_invalidate_only_the_selected_content(self):
        with tempfile.TemporaryDirectory() as temporary:
            control = Path(temporary)
            subprocess.check_call(["git", "clone", "-q", str(self.root), str(control)])
            product_recipe = {"source": "product", "paths": ["package.json"]}
            control_recipe = {"source": "control", "paths": ["package.json"]}
            before = Sources({"product": self.root, "control": control})
            product = before.fingerprint(product_recipe)
            implementation = before.fingerprint(control_recipe)
            (control / "package.json").write_text('{"version":"1","code":"changed control"}')
            subprocess.check_call(["git", "-C", str(control), "-c", "user.name=Fixture",
                                   "-c", "user.email=fixture@example.test", "-c", "commit.gpgsign=false",
                                   "commit", "-qam", "control change"])
            after = Sources({"product": self.root, "control": control})
            self.assertEqual(product, after.fingerprint(product_recipe))
            self.assertNotEqual(implementation, after.fingerprint(control_recipe))

    def test_unknown_source_never_falls_back_to_product(self):
        with self.assertRaises(Refusal):
            Sources({"product": self.root}).fingerprint({"source": "control", "paths": ["package.json"]})

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Blob fixture")
        self.git("config", "user.email", "blob-fixture@example.invalid")
        (self.root / "package.json").write_text('{"version":"1","code":"same"}')
        (self.root / "unrelated").write_text("before")
        self.commit()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.root), *arguments],
                                       stderr=subprocess.PIPE)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "Fixture source")

    def recipe(self):
        return {"paths": ["package.json"], "projects": {
            "package.json": {"format": "json", "set": {"/version": "0.0.0"}}}}

    def test_equivalent_path_enumeration_order_is_canonical(self):
        first = {"paths": ["package.json", "unrelated"]}
        second = {"paths": ["unrelated", "package.json", "unrelated"]}
        self.assertEqual(Source(self.root).fingerprint(first), Source(self.root).fingerprint(second))

    def test_hash_reads_committed_blobs_not_runner_worktree(self):
        before = Source(self.root).fingerprint(self.recipe())
        (self.root / "package.json").write_text("uncommitted runner content")
        self.assertEqual(before, Source(self.root).fingerprint(self.recipe()))

    def test_export_attributes_cannot_change_hashed_or_materialized_input(self):
        (self.root / ".gitattributes").write_text("package.json export-ignore\nunrelated export-subst\n")
        (self.root / "unrelated").write_text("$Format:%H$")
        self.commit()
        snapshot = Source(self.root).snapshot({"paths": ["package.json", "unrelated"]})
        destination = self.root / "materialized"
        snapshot.materialize(destination)
        self.assertEqual((destination / "unrelated").read_bytes(), b"$Format:%H$")
        self.assertTrue((destination / "package.json").is_file())
        self.assertFalse((destination / ".git").exists())

    def test_identity_projection_materializes_equal_usable_documents(self):
        first = Source(self.root).snapshot(self.recipe())
        (self.root / "package.json").write_text('{"version":"2","code":"same"}')
        self.commit()
        second = Source(self.root).snapshot(self.recipe())
        self.assertEqual(first.key, second.key)
        first.materialize(self.root / "first")
        second.materialize(self.root / "second")
        left = (self.root / "first/package.json").read_bytes()
        right = (self.root / "second/package.json").read_bytes()
        self.assertEqual(left, right)
        self.assertEqual(json.loads(left)["version"], "0.0.0")

    def test_unrelated_commit_and_projected_version_preserve_hash(self):
        before = Source(self.root).fingerprint(self.recipe())
        (self.root / "unrelated").write_text("after")
        (self.root / "package.json").write_text('{"version":"2","code":"same"}')
        self.commit()
        self.assertEqual(before, Source(self.root).fingerprint(self.recipe()))

    def test_real_source_change_moves_hash(self):
        before = Source(self.root).fingerprint(self.recipe())
        (self.root / "package.json").write_text('{"version":"1","code":"different"}')
        self.commit()
        self.assertNotEqual(before, Source(self.root).fingerprint(self.recipe()))

    def test_projection_outside_selected_files_refuses(self):
        recipe = self.recipe()
        recipe["paths"] = ["unrelated"]
        with self.assertRaises(Refusal):
            Source(self.root).fingerprint(recipe)

    def test_path_traversal_refuses(self):
        with self.assertRaises(Refusal):
            Source(self.root).fingerprint({"paths": ["../outside"]})

    def test_mode_changes_invalidate(self):
        before = Source(self.root).fingerprint({"paths": ["unrelated"]})
        self.git("update-index", "--chmod=+x", "unrelated")
        self.git("commit", "-qm", "Executable fixture")
        self.assertNotEqual(before, Source(self.root).fingerprint({"paths": ["unrelated"]}))


if __name__ == "__main__":
    unittest.main()
