import unittest
import tempfile
from pathlib import Path

from lib.blob import Refusal, digest
from lib.declaration import Declaration
from lib.execution import Execution
from lib.runner import Runner
from support import Fixture, node, reference


class Preparation(unittest.TestCase):
    def setUp(self):
        self.fixture = Fixture()
        self.fixture.seed()
        self.fixture.value["nodes"].append(node("controller", {}, "tool"))
        for current in self.fixture.value["nodes"]:
            if current["id"] in ("produce", "bind", "deliver"):
                current["prepare"] = [reference("controller", "tool")]

    def test_warm_target_does_not_read_or_prepare_controller(self):
        held = self.fixture.plan()
        self.assertTrue(held["complete"])
        self.assertEqual(held["run"], [])
        self.assertEqual(held["nodes"]["controller"]["state"], "skip")
        self.assertIsNone(held["nodes"]["controller"]["key"])

    def test_miss_activates_preparation_before_execution(self):
        self.fixture.change("bind", "identity", b"new-marker")
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["controller"])
        self.assertEqual(held["nodes"]["bind"]["waiting"], ["controller"])
        self.fixture.finish("controller")
        self.assertEqual(self.fixture.plan()["run"], ["bind"])

    def test_preparation_is_not_a_content_invalidation_input(self):
        before = self.fixture.plan()["nodes"]["bind"]["key"]
        self.fixture.change("controller", "implementation", b"new-controller")
        held = self.fixture.plan()
        self.assertEqual(held["nodes"]["bind"]["key"], before)
        self.assertTrue(held["complete"])

    def test_unknown_output_does_not_activate_consumer_preparation(self):
        self.fixture.value["nodes"].append(node("binding-tool", {}, "tool"))
        self.fixture.value["nodes"][2]["prepare"] = [reference("binding-tool", "tool")]
        self.fixture.change("produce", "source", b"new-source")
        self.assertEqual(self.fixture.plan()["nodes"]["binding-tool"]["state"], "skip")
        self.fixture.finish("controller")
        self.fixture.finish("produce")
        held = self.fixture.plan()
        self.assertTrue(held["complete"])
        self.assertEqual(held["nodes"]["binding-tool"]["state"], "skip")

    def test_completion_check_does_not_activate_cached_consumer_preparation(self):
        self.fixture.change("check", "implementation", b"new-check")
        held = self.fixture.plan()
        self.assertEqual(held["run"], ["check"])
        self.assertFalse(held["complete"])
        self.assertEqual(held["nodes"]["controller"]["state"], "skip")

    def test_execution_receives_preparation_artifact(self):
        self.fixture.change("bind", "identity", b"new-marker")
        self.fixture.finish("controller")
        execution = Execution(Declaration(self.fixture.value), self.fixture.inventory,
                              "https://blobs.example.test")
        request = execution.prepare("bind")["request"]
        tool = request["producers"]["controller"]["outputs"]["tool"]
        self.assertEqual(tool["digest"], digest(b"controller"))
        self.assertNotIn("controller", request["inputs"])

    def test_preparation_cycles_refuse_even_when_business_is_cached(self):
        self.fixture.value["nodes"][-1]["prepare"] = [reference("bind", "bound")]
        with self.assertRaises(Refusal):
            self.fixture.plan()

    def test_explicit_preparation_target_is_not_suppressed(self):
        self.fixture.value["targets"].append("controller")
        self.assertEqual(self.fixture.plan()["run"], ["controller"])

    def test_preparation_type_is_closed(self):
        self.fixture.value["nodes"][0]["prepare"] = "controller"
        with self.assertRaises(Refusal):
            self.fixture.plan()

    def test_runner_prepares_once_and_warm_invocation_does_nothing(self):
        self.fixture.change("bind", "identity", b"new-marker")
        calls = []

        def invoke(request, seat):
            calls.append(request["node"])
            path = seat / "artifact"
            path.write_bytes(request["node"].encode())
            label = "tool" if request["node"] == "controller" else "bound"
            return {"key": request["key"], "outputs": {label: str(path)}}

        with tempfile.TemporaryDirectory() as root:
            execution = Execution(Declaration(self.fixture.value), self.fixture.inventory,
                                  "https://blob.example")
            runner = Runner(execution, Path(root))
            runner.run("bind", invoke, lambda identity: invoke)
            runner.run("bind", invoke, lambda identity: self.fail("warm preparation"))
        self.assertEqual(calls, ["controller", "bind"])

    def test_unavailable_preparation_never_starts_business_execution(self):
        self.fixture.change("bind", "identity", b"new-marker")
        execution = Execution(Declaration(self.fixture.value), self.fixture.inventory,
                              "https://blob.example")
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaises(Refusal):
                Runner(execution, root).run("bind", lambda *args: self.fail("business invoked"))
