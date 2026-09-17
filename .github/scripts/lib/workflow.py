import copy
import os
import subprocess

from .acceptance import declaration
from .blob import Refusal, decode


def read(route):
    environment = dict(os.environ)
    environment.pop("GH_DEBUG", None)
    environment.update(GH_HOST="github.com", GH_PROMPT_DISABLED="1")
    result = subprocess.run(
        ["gh", "api", "--hostname", "github.com", "--method", "GET",
         "-H", "Accept: application/vnd.github+json",
         "-H", "X-GitHub-Api-Version: 2026-03-10", route],
        env=environment, capture_output=True, timeout=30, check=False)
    if result.returncode:
        raise Refusal("cannot read independent GitHub acceptance evidence")
    return decode(result.stdout)


def observe(value, contract, reader=read):
    declaration(value, contract)
    context = value["context"]
    route = f"repos/{context['repository']}/actions/runs/{context['run']}/attempts/{context['attempt']}"
    run = reader(route)
    if not isinstance(run, dict):
        raise Refusal("GitHub acceptance run is not an object")
    expected = {"id": context["run"], "run_attempt": context["attempt"],
                "head_sha": context["control"], "event": "workflow_dispatch"}
    if any(run.get(key) != held for key, held in expected.items()):
        raise Refusal("GitHub run does not match the selected control and attempt")
    for field in ("repository", "head_repository"):
        if not isinstance(run.get(field), dict) or run[field].get("full_name") != context["repository"]:
            raise Refusal("GitHub acceptance run belongs to another repository")
    path = run.get("path", "")
    if not isinstance(path, str) or path.split("@", 1)[0] != context["workflow"]:
        raise Refusal("GitHub acceptance run used another workflow")
    required = set(contract["jobs"].values())
    found = {}
    total = None
    count = 0
    for page in range(1, 101):
        held = reader(route + f"/jobs?per_page=100&page={page}")
        if not isinstance(held, dict) or not isinstance(held.get("jobs"), list):
            raise Refusal("GitHub acceptance job listing is invalid")
        if type(held.get("total_count")) is not int or held["total_count"] < 0:
            raise Refusal("GitHub acceptance job count is invalid")
        if total is None:
            total = held["total_count"]
        if total != held["total_count"]:
            raise Refusal("GitHub acceptance job listing changed during readback")
        for job in held["jobs"]:
            if not isinstance(job, dict):
                raise Refusal("GitHub acceptance job is invalid")
            name = job.get("name")
            if not isinstance(name, str):
                raise Refusal("GitHub acceptance job name is invalid")
            if name not in required:
                continue
            if name in found:
                raise Refusal("GitHub acceptance job name is ambiguous")
            if job.get("run_id") != context["run"] or job.get("head_sha") != context["control"]:
                raise Refusal("GitHub acceptance job belongs to another execution")
            if job.get("status") != "completed" or job.get("conclusion") != "success":
                raise Refusal("required acceptance job did not succeed")
            found[name] = job
        count += len(held["jobs"])
        if len(held["jobs"]) < 100:
            break
    else:
        raise Refusal("GitHub acceptance job listing exceeded its bound")
    if count != total or set(found) != required:
        raise Refusal("GitHub acceptance job evidence is incomplete")
    return {key: copy.deepcopy(value[key]) for key in
            ("context", "source", "configuration", "contract")} | {"conclusion": "success"}
