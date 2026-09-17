import copy
import subprocess
import unittest
from unittest.mock import patch

from lib.blob import Refusal
from lib.workflow import observe, read
import test_acceptance


class Workflow(unittest.TestCase):
    def setUp(self):
        fixture = test_acceptance.Acceptance()
        fixture.setUp()
        self.value, self.contract = fixture.value, fixture.contract
        self.run = {"id": 17, "run_attempt": 1, "head_sha": "a" * 40,
                    "event": "workflow_dispatch", "path": ".github/workflows/ship.yml@main",
                    "repository": {"full_name": "PerishLab/plumb"},
                    "head_repository": {"full_name": "PerishLab/plumb"}}
        self.jobs = [{"name": name, "run_id": 17, "head_sha": "a" * 40,
                      "status": "completed", "conclusion": "success"}
                     for name in self.contract["jobs"].values()]
        self.routes = []

    def reader(self, route):
        self.routes.append(route)
        if "/jobs?" in route:
            return {"total_count": len(self.jobs), "jobs": copy.deepcopy(self.jobs)}
        return copy.deepcopy(self.run)

    def test_observes_exact_attempt_and_all_required_jobs(self):
        observed = observe(self.value, self.contract, self.reader)
        self.assertEqual(observed["conclusion"], "success")
        self.assertTrue(all("/attempts/1" in route for route in self.routes))

    def test_other_run_control_event_or_repository_refuses(self):
        original = copy.deepcopy(self.run)
        for field, wrong in (("id", 18), ("run_attempt", 2), ("head_sha", "b" * 40),
                             ("event", "pull_request"), ("path", ".github/workflows/other.yml"),
                             ("head_repository", {"full_name": "external/plumb"})):
            self.run = dict(original, **{field: wrong})
            with self.subTest(field=field), self.assertRaises(Refusal):
                observe(self.value, self.contract, self.reader)

    def test_missing_duplicate_skipped_or_failed_jobs_refuse(self):
        original = copy.deepcopy(self.jobs)
        for jobs in (original[:1], original + original[:1],
                     [dict(original[0], conclusion="skipped"), original[1]],
                     [dict(original[0], conclusion="failure"), original[1]],
                     [dict(original[0], run_id=18), original[1]]):
            self.jobs = jobs
            with self.subTest(jobs=jobs), self.assertRaises(Refusal):
                observe(self.value, self.contract, self.reader)

    def test_truncated_job_listing_is_not_success(self):
        def reader(route):
            return {"jobs": self.jobs, "total_count": 200} if "/jobs?" in route else self.run
        with self.assertRaises(Refusal):
            observe(self.value, self.contract, reader)

    def test_malformed_job_names_refuse(self):
        for name in (None, [], {}):
            self.jobs[0]["name"] = name
            with self.subTest(name=name), self.assertRaises(Refusal):
                observe(self.value, self.contract, self.reader)

    def test_provider_failure_does_not_expose_stderr(self):
        result = subprocess.CompletedProcess([], 1, b"", b"private credential value")
        with patch("lib.workflow.subprocess.run", return_value=result), self.assertRaises(Refusal) as caught:
            read("repos/PerishLab/plumb/actions/runs/17")
        self.assertNotIn("private credential value", str(caught.exception))
