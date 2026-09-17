import copy
import unittest
from pathlib import Path

from dispatch_mode import select
from lib.blob import Refusal, fingerprint
from lib.suite import PLATFORMS, graph, prepare, request, validate
from support import Memory
from lib.inventory import Inventory


class Sources:
    def fingerprint(self, recipe):
        return fingerprint(recipe)


class Suite(unittest.TestCase):
    def setUp(self):
        self.value = {"schema": "plumb.source-suite-request/v1", "source": "a" * 40,
                      "configuration": "b" * 64, "toolchains": {
                          name: {"channel": "1.96.1", "rustc": "rustc 1.96.1 (fixture)",
                                 "cargo": "cargo 1.96.1 (fixture)"} for name in PLATFORMS}}
        self.context = {"repository": "PerishLab/plumb", "control": "c" * 40,
                        "workflow": ".github/workflows/ship.yml", "run": 1, "attempt": 1}
        self.store = Memory()
        self.inventory = Inventory(self.store)

    def plan(self):
        return prepare(self.value, self.context, Sources(), self.inventory, "https://inventory.example.test")

    def test_source_mode_is_explicit_and_excludes_publication(self):
        self.assertEqual(select("a" * 40, "b" * 64, "", "", ""), "source")
        self.assertEqual(select("", "b" * 64, "v1.0.0", "v1.0.0", "PerishLab/plumb"), "ship")
        for values in (("main", "b" * 64, "", "", ""),
                       ("a" * 40, "b" * 64, "v1.0.0", "", ""),
                       ("", "b" * 64, "", "", "")):
            with self.subTest(values=values), self.assertRaises(Refusal):
                select(*values)

    def test_three_platforms_are_pinned_and_independently_keyed(self):
        before = validate(self.plan())
        self.assertEqual(set(before["work"]), {"linux", "windows", "macos"})
        self.assertEqual({work["state"] for work in before["work"].values()}, {"run"})
        self.value["toolchains"]["windows"]["rustc"] += " changed"
        after = self.plan()
        for name in PLATFORMS:
            same = before["work"][name]["request"]["key"] == after["work"][name]["request"]["key"]
            self.assertEqual(same, name != "windows")

    def test_second_run_reuses_all_outputs_without_writes(self):
        before = self.plan()
        for name, held in before["work"].items():
            binary = self.inventory.upload((name + " binary").encode())
            receipt = self.inventory.upload((name + " receipt").encode())
            self.inventory.complete(held["request"]["key"], {"binary": binary}, {"receipt": receipt})
        self.store.writes.clear()
        self.context["attempt"] = 2
        after = self.plan()
        self.assertEqual({work["state"] for work in after["work"].values()}, {"reuse"})
        self.assertEqual(self.store.writes, [])

    def test_no_missing_platform_floating_tool_or_injected_command(self):
        missing = copy.deepcopy(self.value)
        missing["toolchains"].pop("macos")
        floating = copy.deepcopy(self.value)
        floating["toolchains"]["linux"]["channel"] = "stable"
        for value in (missing, floating, dict(self.value, command="publish")):
            with self.subTest(value=value), self.assertRaises(Refusal):
                request(value)

    def test_graph_uses_existing_snapshot_and_action_contract(self):
        for node in graph(self.value)["nodes"]:
            self.assertEqual(node["execution"]["entry"], "suite.native")
            self.assertEqual(node["execution"]["materialize"], ["source"])
            self.assertEqual(node["inputs"]["implementation"]["tree"]["source"], "control")
            self.assertEqual(node["inputs"]["configuration"], self.value["configuration"])

    def test_workflow_has_no_candidate_writer_or_publication_lane(self):
        held = (Path(__file__).parents[2] / "workflows/ship.yml").read_text()
        common = held.split("\njobs:", 1)[0]
        candidate = held.split("\n  source-suite:\n", 1)[1].split("\n  source-collect:\n", 1)[0]
        for text in (common, candidate):
            for forbidden in ("S3_ACCESS_KEY", "S3_SECRET_KEY", "GH_TOKEN:", "packages: write"):
                self.assertNotIn(forbidden, text)
        self.assertIn("artifact-ids: ${{ needs.resolve.outputs.suite_artifact }}", candidate)
        self.assertIn("persist-credentials: false", candidate)
        self.assertIn("default: ''", common)
        self.assertNotIn("\n  select:\n", held)
