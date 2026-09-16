import tempfile
import unittest
from pathlib import Path

from execute import Executor
from lib.blob import Refusal
from lib.declaration import Declaration
from lib.execution import Execution
from lib.inputs import Inputs
from lib.runner import Runner
from lib.snapshot import Snapshot
from support import Fixture


BRIDGE = '''import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--request", type=Path)
parser.add_argument("--result", type=Path)
args = parser.parse_args()
request = json.loads(args.request.read_text())
assert request["entry"] == "fixture.produce"
output = args.result.parent / "content"
output.write_bytes(b"opaque subprocess output")
args.result.write_text(json.dumps({"key": request["key"], "outputs": {"content": str(output)}}))
'''


class Process(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bridge = self.root / "business.py"
        self.bridge.write_text(BRIDGE)
        self.fixture = Fixture()
        self.runner = Runner(Execution(Declaration(self.fixture.value), self.fixture.inventory,
                                       "https://blob.example"), self.root / "state")

    def test_subprocess_roundtrip_and_warm_skip(self):
        executor = Executor(self.bridge, self.root, 10)
        self.assertEqual(self.runner.run("produce", executor)["state"], "complete")
        self.bridge.write_text("raise AssertionError('a cache hit started the executor')")
        self.assertEqual(self.runner.run("produce", executor)["state"], "reuse")

    def test_nonzero_exit_never_accepts_candidate(self):
        self.bridge.write_text(BRIDGE + "\nraise SystemExit(3)\n")
        with self.assertRaises(Refusal):
            self.runner.run("produce", Executor(self.bridge, self.root, 10))
        self.assertEqual(self.fixture.store.writes, [])
        self.assertEqual(list(self.root.rglob("result.json")), [])
        self.bridge.write_text(BRIDGE)
        self.assertEqual(self.runner.run("produce", Executor(self.bridge, self.root, 10))["state"],
                         "complete")

    def test_stale_candidate_is_not_a_successful_result(self):
        self.bridge.write_text(BRIDGE + "\nraise SystemExit(3)\n")
        with self.assertRaises(Refusal):
            self.runner.run("produce", Executor(self.bridge, self.root, 10))
        self.bridge.write_text("pass\n")
        with self.assertRaises(OSError):
            self.runner.run("produce", Executor(self.bridge, self.root, 10))
        self.assertEqual(self.fixture.store.writes, [])

    def input_executor(self):
        snapshot = Snapshot({"paths": ["source"]}, {"source": ("100644", b"normalized")})
        self.fixture.value["nodes"][0]["inputs"]["source"] = snapshot.key
        self.runner = Runner(Execution(Declaration(self.fixture.value), self.fixture.inventory,
                                       "https://blob.example"), self.root / "state")
        self.bridge.write_text(BRIDGE.replace('output.write_bytes(b"opaque subprocess output")',
            'held = request["materialized"]["source"]\n'
            'assert held["key"] == request["inputs"]["source"]["digest"]\n'
            'assert (Path(held["root"]) / "source").read_bytes() == b"normalized"\n'
            'output.write_bytes(b"verified input")'))
        return Executor(self.bridge, self.root, 10, Inputs({"source": snapshot}))

    def test_executor_consumes_verified_input_and_warm_skip_does_not_materialize(self):
        executor = self.input_executor()
        self.assertEqual(self.runner.run("produce", executor)["state"], "complete")
        attempts = list(self.root.rglob("inputs-*"))
        self.assertEqual(len(attempts), 1)
        self.assertEqual(self.runner.run("produce", executor)["state"], "reuse")
        self.assertEqual(list(self.root.rglob("inputs-*")), attempts)

    def test_wrong_input_key_refuses_before_business_or_materialization(self):
        executor = self.input_executor()
        executor.inputs = Inputs({"source": Snapshot({}, {"source": ("100644", b"wrong")})})
        with self.assertRaises(Refusal):
            self.runner.run("produce", executor)
        self.assertEqual(list(self.root.rglob("inputs-*")), [])
        self.assertEqual(self.fixture.store.writes, [])

    def test_failed_execution_retries_with_fresh_inputs(self):
        executor = self.input_executor()
        original = self.bridge.read_text()
        self.bridge.write_text(original + '\n(Path(held["root"]) / "source").write_bytes(b"dirty")\nraise SystemExit(3)\n')
        with self.assertRaises(Refusal):
            self.runner.run("produce", executor)
        self.bridge.write_text(original)
        self.assertEqual(self.runner.run("produce", executor)["state"], "complete")
        self.assertEqual(len(list(self.root.rglob("inputs-*"))), 2)

    def test_materialization_selection_does_not_copy_implementation_inputs(self):
        class Source:
            def __init__(self):
                self.calls = []

            def snapshot(self, recipe):
                self.calls.append(recipe)
                return recipe

        source = Source()
        declaration = {"nodes": [{"id": "produce", "inputs": {
            "source": {"tree": {"paths": ["source"]}},
            "implementation": {"tree": {"paths": ["implementation"]}}},
            "execution": {"materialize": ["source"]}}]}
        selected = Inputs.select(declaration, "produce", source)
        self.assertEqual(set(selected.snapshots), {"source"})
        self.assertEqual(source.calls, [{"paths": ["source"]}])


if __name__ == "__main__":
    unittest.main()
