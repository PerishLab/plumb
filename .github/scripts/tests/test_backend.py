import unittest

from lib.backend import Backend
from lib.blob import Refusal
from lib.declaration import Declaration
from support import Fixture, node, reference


def backend():
    return Backend({"schema": "plumb.blob-backend/v1", "jobs": [
        {"id": "first", "needs": [], "capabilities": ["native", "network"]},
        {"id": "second", "needs": ["first"], "capabilities": ["native", "network"]},
        {"id": "third", "needs": ["second"], "capabilities": ["network"]},
    ], "placement": {"fixture.produce": "first", "fixture.independent": "first",
                      "fixture.check": "second", "fixture.bind": "second",
                      "fixture.deliver": "third"}})


class Jobs(unittest.TestCase):
    def preparation(self):
        fixture = Fixture()
        fixture.seed()
        fixture.value["nodes"].append(node("controller", {}, "tool"))
        fixture.value["nodes"][2]["prepare"] = [reference("controller", "tool")]
        held = backend()
        held.placement["fixture.controller"] = "second"
        held.jobs["second"]["runners"] = {"native": "linux", "network": "linux"}
        return fixture, held

    def test_preparation_is_local_not_an_extra_matrix_runner(self):
        fixture, held = self.preparation()
        fixture.change("bind", "identity", b"new-marker")
        graph = Declaration(fixture.value)
        self.assertEqual(held.matrix(graph, fixture.plan(), "second"),
                         {"include": [{"node": "bind", "runner": "linux"}]})

    def test_cached_consumer_starts_no_preparation_matrix(self):
        fixture, held = self.preparation()
        self.assertEqual(held.matrix(Declaration(fixture.value), fixture.plan(), "second"),
                         {"include": []})

    def test_cross_job_preparation_refuses(self):
        fixture, held = self.preparation()
        held.placement["fixture.controller"] = "first"
        with self.assertRaises(Refusal):
            held.place(Declaration(fixture.value))

    def test_preparation_cannot_hide_same_job_content_dependency(self):
        fixture, held = self.preparation()
        fixture.value["nodes"][2]["inputs"]["tool"] = reference("controller", "tool")
        with self.assertRaises(Refusal):
            held.place(Declaration(fixture.value))

    def test_empty_plan_starts_no_business_jobs(self):
        fixture = Fixture()
        fixture.seed()
        projected = backend().project(Declaration(fixture.value), fixture.plan())
        self.assertFalse(any(job["needed"] for job in projected["jobs"].values()))

    def test_cold_graph_defers_unknown_inputs(self):
        fixture = Fixture()
        projected = backend().project(Declaration(fixture.value), fixture.plan())
        self.assertEqual(projected["jobs"]["first"]["run"], ["produce", "independent"])
        self.assertEqual(projected["jobs"]["second"]["pending"], ["check", "bind"])
        self.assertEqual(projected["jobs"]["third"]["pending"], ["deliver"])

    def test_unrepresentable_graph_refuses_without_publishing(self):
        fixture = Fixture()
        fixture.value["nodes"][4]["execution"]["capability"] = "unknown"
        with self.assertRaises(Refusal):
            backend().project(Declaration(fixture.value), fixture.plan())
        self.assertEqual(fixture.store.writes, [])

    def test_validation_only_reuses_publication(self):
        fixture = Fixture()
        fixture.seed()
        fixture.change("check", "implementation", b"changed")
        projected = backend().project(Declaration(fixture.value), fixture.plan())
        self.assertEqual(projected["jobs"]["second"]["run"], ["check"])
        self.assertFalse(projected["jobs"]["third"]["needed"])
        self.assertFalse(fixture.plan()["complete"])

    def test_backend_barrier_is_not_reported_as_business_dependency(self):
        fixture = Fixture()
        fixture.value["nodes"] = [fixture.value["nodes"][4]]
        fixture.value["targets"] = ["independent"]
        held = Backend({"schema": "plumb.blob-backend/v1", "jobs": [
            {"id": "prepare", "needs": [], "capabilities": ["native"]},
            {"id": "publish", "needs": ["prepare"], "capabilities": ["network"]},
        ], "placement": {"fixture.independent": "publish"}})
        projected = held.project(Declaration(fixture.value), fixture.plan())
        self.assertEqual(projected["jobs"]["publish"]["backend_barriers"], ["prepare"])

    def test_job_order_does_not_choose_entry_placement(self):
        fixture = Fixture()
        held = backend()
        held.jobs = dict(reversed(list(held.jobs.items())))
        self.assertEqual(held.place(Declaration(fixture.value))["produce"], "first")

    def test_same_job_dependency_refuses(self):
        held = backend()
        held.placement["fixture.check"] = "first"
        with self.assertRaises(Refusal):
            held.place(Declaration(Fixture().value))

    def test_missing_entry_refuses(self):
        held = backend()
        del held.placement["fixture.bind"]
        with self.assertRaises(Refusal):
            held.place(Declaration(Fixture().value))

    def test_parallel_branches_have_explicit_placement(self):
        fixture = Fixture()
        fixture.value["nodes"] = [fixture.value["nodes"][0], fixture.value["nodes"][2]]
        fixture.value["targets"] = ["bind"]
        held = Backend({"schema": "plumb.blob-backend/v1", "jobs": [
            {"id": "unrelated", "needs": [], "capabilities": ["native"]},
            {"id": "build", "needs": [], "capabilities": ["native"]},
            {"id": "bind", "needs": ["build"], "capabilities": ["native"]},
        ], "placement": {"fixture.produce": "build", "fixture.bind": "bind"}})
        self.assertEqual(held.place(Declaration(fixture.value))["produce"], "build")


if __name__ == "__main__":
    unittest.main()
