import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from lib.blob import decode, digest, encode
from lib.source import Source


class Input(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def run_command(self, root, *command, environment=None):
        result = subprocess.run(command, cwd=root, env=environment, capture_output=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        return result.stdout

    def fixture(self, name, version):
        root = self.root / name
        (root / "src").mkdir(parents=True)
        (root / "Cargo.toml").write_text(
            f'[package]\nname="fixture"\nversion="{version}"\nedition="2021"\n')
        (root / "src/main.rs").write_text('fn main() { println!("{}", env!("CARGO_PKG_VERSION")); }')
        self.run_command(root, "cargo", "generate-lockfile", "--offline")
        self.run_command(root, "git", "init", "-q")
        self.run_command(root, "git", "add", ".")
        self.run_command(root, "git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.test",
                         "-c", "commit.gpgsign=false", "commit", "-qm", "fixture")
        return root

    def recipe(self, root):
        recipe = {"paths": ["Cargo.toml", "Cargo.lock", "src"], "projects": {
            "Cargo.toml": {"format": "toml", "set": {"/package/version": "0.0.0"}},
            "Cargo.lock": {"format": "toml", "set": {"/package/0/version": "0.0.0"}}}}
        recipe["patches"] = {}
        for name in recipe["projects"]:
            body = (root / name).read_bytes()
            line = next(line for line in body.splitlines() if line.startswith(b"version") and b'"' in line)
            start = body.index(line) + line.index(b'"')
            end = body.index(b'"', start + 1) + 1
            result = body[:start] + b'"0.0.0"' + body[end:]
            recipe["patches"][name] = {"source": digest(body), "result": digest(result),
                                       "edits": [[start, end, '"0.0.0"']]}
        return recipe

    def build(self, root):
        environment = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"), CARGO_INCREMENTAL="0",
                           RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
                           CARGO_ENCODED_RUSTFLAGS=f"--remap-path-prefix={root}=/plumb/source")
        self.run_command(root, "cargo", "build", "--release", "--locked", "--offline", environment=environment)
        path = root / "target/release" / ("fixture.exe" if os.name == "nt" else "fixture")
        self.assertEqual(self.run_command(root, str(path)).strip(), b"0.0.0")
        return path.read_bytes()

    def test_independent_cold_builds_use_the_same_hashed_materialization(self):
        first = self.fixture("first", "1.2.3")
        second = self.fixture("second", "4.5.6-beta.2")
        left = Source(first).snapshot(self.recipe(first))
        right = Source(second).snapshot(self.recipe(second))
        self.assertEqual(left.key, right.key)
        left.materialize(self.root / "left")
        right.materialize(self.root / "right")
        self.assertEqual(self.build(self.root / "left"), self.build(self.root / "right"))
        self.assertIn('version="1.2.3"', (first / "Cargo.toml").read_text())

    def test_cli_checks_planned_key_before_creating_destination(self):
        root = self.fixture("fixture", "1.2.3")
        recipe = self.root / "recipe.json"
        recipe.write_bytes(encode(self.recipe(root)))
        script = Path(__file__).resolve().parents[1] / "input.py"
        command = [sys.executable, "-B", str(script), "--root", str(root), "--recipe", str(recipe)]
        planned = decode(self.run_command(root, *command))
        destination = self.root / "materialized"
        rejected = subprocess.run([*command, "--expect", "0" * 64, "--destination", str(destination)],
                                  capture_output=True, timeout=30)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertFalse(destination.exists())
        result = decode(self.run_command(root, *command, "--expect", planned["key"],
                                         "--destination", str(destination)))
        self.assertEqual(result, planned)
