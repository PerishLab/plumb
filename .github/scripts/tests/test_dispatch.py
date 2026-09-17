import copy
import contextlib
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from dispatch import JOBS, prepare
from lib.blob import Refusal, digest, encode
from lib.planner import plan
from local import checkout, main as local_main
from support import Memory, node


class Dispatch(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "product"
        self.root.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Dispatch fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("remote", "add", "origin", "https://example.invalid/product.git")
        (self.root / "source").write_text("committed")
        self.git("add", "source")
        self.git("commit", "-qm", "fixture")
        self.commit = self.git("rev-parse", "HEAD").decode().strip()
        self.value = {
            "schema": "plumb.ship-dispatch/v1", "marker": "v1.0.0-beta.1", "commit": self.commit,
            "graph": {"schema": "plumb.blob-graph/v1", "nodes": [
                node("deliver", {"source": digest(b"input")}, "receipt", "network")],
                "targets": ["deliver"]},
            "backend": {"schema": "plumb.blob-backend/v1", "jobs": [
                {"id": job, "needs": list(JOBS[:index]), "capabilities": ["network"],
                 "runners": {"network": "linux"}}
                for index, job in enumerate(JOBS)], "placement": {"fixture.deliver": "publish"}},
        }
        self.environment = patch.dict(os.environ, {"PLUMB_RELEASE_MARKER": self.value["marker"]})
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.root), *arguments], stderr=subprocess.PIPE)

    def test_declared_graph_uses_the_central_finite_backend(self):
        graph, backend = prepare(self.value, self.root, self.root)
        self.assertEqual(backend.place(graph), {"deliver": "publish"})

    def test_marker_and_commit_must_both_match(self):
        for field, value in (("marker", "v2.0.0"), ("commit", "a" * 40)):
            with self.subTest(field=field), self.assertRaises(Refusal):
                changed = copy.deepcopy(self.value)
                changed[field] = value
                prepare(changed, self.root, self.root)

    def test_arbitrary_jobs_and_barriers_are_not_a_second_scheduler(self):
        changed = copy.deepcopy(self.value)
        changed["backend"]["jobs"].append({"id": "extra", "needs": [], "capabilities": ["network"]})
        with self.assertRaises(Refusal):
            prepare(changed, self.root, self.root)
        changed = copy.deepcopy(self.value)
        changed["backend"]["jobs"][-1]["needs"] = []
        with self.assertRaises(Refusal):
            prepare(changed, self.root, self.root)

    def test_local_nodes_have_independent_clean_checkouts_and_original_authority(self):
        (self.root / "source").write_text("caller mutation")
        first = checkout(self.root, self.directory.name)
        (first / "source").write_text("first action mutation")
        second = checkout(self.root, self.directory.name)
        self.assertNotEqual(first, second)
        self.assertEqual((second / "source").read_text(), "committed")
        for held in (first, second):
            actual = subprocess.check_output(["git", "-C", str(held), "remote", "get-url", "origin"])
            self.assertEqual(actual.decode().strip(), "https://example.invalid/product.git")
            actual = subprocess.check_output(["git", "-C", str(held), "rev-parse", "HEAD"])
            self.assertEqual(actual.decode().strip(), self.commit)

    def local(self, store, execute, runners='["linux"]'):
        declaration = Path(self.directory.name) / "declaration.json"
        declaration.write_bytes(encode(self.value))
        arguments = ["local.py", "--declaration", str(declaration), "--root", str(self.root),
                     "--control", str(self.root), "--runners", runners]
        with patch("sys.argv", arguments), patch("local.R2.environment", return_value=store), \
             patch("local.perform", side_effect=execute), \
             patch.dict(os.environ, {"RUNNER_TEMP": self.directory.name,
                                     "PLUMB_WORKFLOW_INVENTORY_URL": "https://blob.invalid"}), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            local_main()
        return output.getvalue()

    def test_local_unavailable_runner_stays_incomplete_without_execution(self):
        store = Memory()
        with self.assertRaisesRegex(Refusal, "pending nodes: deliver"):
            self.local(store, lambda *args: self.fail("unavailable runner executed"), '["macos"]')
        self.assertEqual(store.writes, [])

    def test_local_retry_keeps_completed_nodes_and_hits_do_not_prepare(self):
        self.value["graph"]["nodes"].append(node("second", {"source": digest(b"second")}, "receipt", "network"))
        self.value["graph"]["targets"].append("second")
        self.value["backend"]["placement"]["fixture.second"] = "publish"
        store, calls = Memory(), []

        def execute(args, value, graph, inventory):
            calls.append(args.node)
            if calls == ["deliver", "second"]:
                raise Refusal("fixture execution failed")
            key = plan(graph, inventory)["nodes"][args.node]["key"]
            inventory.complete(key, {"receipt": inventory.upload(args.node.encode())})

        with self.assertRaisesRegex(Refusal, "fixture execution failed"):
            self.local(store, execute)
        self.assertEqual(calls, ["deliver", "second"])
        self.assertIn('"complete":true', self.local(store, execute))
        self.assertEqual(calls, ["deliver", "second", "second"])
        with patch("local.checkout", side_effect=AssertionError("hit prepared a checkout")):
            self.assertIn('"executed":[]', self.local(store, execute))
