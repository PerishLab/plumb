import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from lib.blob import Refusal, Unknown
from lib.declaration import Declaration
from lib.execution import Execution
from lib.runner import Runner
from support import Fixture


class Recovery(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.fixture = Fixture()
        self.calls = []

    def runner(self):
        execution = Execution(Declaration(self.fixture.value), self.fixture.inventory,
                              "https://blob.example")
        return Runner(execution, Path(self.directory.name))

    def invoke(self, request, seat):
        self.calls.append(request)
        path = seat / "content"
        path.write_bytes(b"unchanged content")
        return {"key": request["key"], "outputs": {"content": str(path)}}

    def test_repeated_execution_is_skipped(self):
        self.assertEqual(self.runner().run("produce", self.invoke)["state"], "complete")
        self.assertEqual(self.runner().run("produce", self.invoke)["state"], "reuse")
        self.assertEqual(len(self.calls), 1)

    def test_failed_record_retries_without_external_execution(self):
        with patch.object(self.fixture.inventory, "upload", side_effect=Unknown("unavailable")):
            with self.assertRaises(Unknown):
                self.runner().run("produce", self.invoke)
        self.assertEqual(self.fixture.store.writes, [])
        self.assertEqual(self.runner().run("produce", self.invoke)["state"], "complete")
        self.assertEqual(len(self.calls), 1)

    def test_failed_executor_does_not_leave_success_evidence(self):
        def failed(request, seat):
            self.invoke(request, seat)
            raise Refusal("destination is unknown; executor must reconcile on retry")

        with self.assertRaises(Refusal):
            self.runner().run("produce", failed)
        self.assertEqual(list(Path(self.directory.name).rglob("result.json")), [])
        self.runner().run("produce", self.invoke)
        self.assertEqual(len(self.calls), 2)

    def test_changed_key_does_not_adopt_prior_local_result(self):
        self.runner().run("produce", self.invoke)
        self.fixture.change("produce", "source", b"changed")
        self.runner().run("produce", self.invoke)
        self.assertEqual(len(self.calls), 2)

    def test_wrong_executor_key_refuses_before_upload(self):
        def wrong(request, seat):
            result = self.invoke(request, seat)
            result["key"] = "a" * 64
            return result

        with self.assertRaises(Refusal):
            self.runner().run("produce", wrong)
        self.assertEqual(self.fixture.store.writes, [])

    def test_corrupt_retained_result_does_not_repeat_publication(self):
        with patch.object(self.fixture.inventory, "upload", side_effect=Unknown("unavailable")):
            with self.assertRaises(Unknown):
                self.runner().run("produce", self.invoke)
        receipt = next(Path(self.directory.name).rglob("result.json"))
        receipt.write_bytes(b"corrupt")
        with self.assertRaises(Refusal):
            self.runner().run("produce", self.invoke)
        self.assertEqual(len(self.calls), 1)


if __name__ == "__main__":
    unittest.main()
