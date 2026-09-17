import copy
import re

from .blob import Refusal, fingerprint, object_fields, sha
from .declaration import Declaration
from .planner import plan


REPOSITORY = "PerishLab/plumb"
WORKFLOW = ".github/workflows/ship.yml"
SCHEMA = "plumb.source-acceptance/v1"


def commit(value):
    if not isinstance(value, str) or not re.fullmatch("[0-9a-f]{40}", value):
        raise Refusal("source acceptance requires a full lowercase commit SHA")
    return value


def intent(source, configuration, contract, context):
    object_fields(context, ("repository", "control", "workflow", "run", "attempt"))
    if context["repository"] != REPOSITORY or context["workflow"] != WORKFLOW:
        raise Refusal("source acceptance requires the central Plumb workflow")
    commit(context["control"])
    for field in ("run", "attempt"):
        if type(context[field]) is not int or context[field] <= 0:
            raise Refusal("source acceptance requires an exact run and attempt")
    return {"schema": SCHEMA, "source": commit(source),
            "configuration": sha(configuration), "contract": sha(contract),
            "context": copy.deepcopy(context)}


def validate(value):
    object_fields(value, ("schema", "source", "configuration", "contract", "context"))
    expected = intent(value["source"], value["configuration"], value["contract"], value["context"])
    if value != expected:
        raise Refusal("unsupported source acceptance intent")
    return value


def declaration(value, contract):
    validate(value)
    if fingerprint(contract) != value["contract"]:
        raise Refusal("acceptance contract differs from its selected digest")
    object_fields(contract, ("schema", "source", "configuration", "control", "graph", "checks", "jobs"))
    if contract["schema"] != "plumb.source-suite/v1":
        raise Refusal("unsupported source suite contract")
    for field in ("source", "configuration"):
        if contract[field] != value[field]:
            raise Refusal("suite contract differs from its selected " + field)
    if contract["control"] != value["context"]["control"]:
        raise Refusal("suite contract differs from its approved control")
    graph = Declaration(contract["graph"])
    checks = contract["checks"]
    if not isinstance(checks, dict) or not checks:
        raise Refusal("source suite requires explicit acceptance checks")
    for check, node in checks.items():
        if (not isinstance(check, str) or not check.strip()
                or not isinstance(node, str) or node not in graph.targets):
            raise Refusal("suite checks must name required graph targets")
    if set(checks.values()) != set(graph.targets):
        raise Refusal("every suite target requires a named acceptance check")
    jobs = contract["jobs"]
    if not isinstance(jobs, dict) or set(jobs) != set(checks):
        raise Refusal("every acceptance check requires an observed workflow job")
    if any(not isinstance(job, str) or not job.strip() for job in jobs.values()):
        raise Refusal("acceptance job names must be nonblank")
    return graph


def collect(value, contract, inventory, observation):
    graph = declaration(value, contract)
    object_fields(observation, ("context", "source", "configuration", "contract", "conclusion"))
    if observation != {key: copy.deepcopy(value[key]) for key in
                       ("context", "source", "configuration", "contract")} | {"conclusion": "success"}:
        raise Refusal("independent workflow observation does not bind this successful acceptance")
    planned = plan(graph, inventory)
    if not planned["complete"]:
        raise Refusal("source suite has incomplete declared checks")
    checks = {}
    for name, identity in sorted(contract["checks"].items()):
        node = planned["nodes"][identity]
        checks[name] = {"node": identity, "key": node["key"],
                        "outputs": node["outputs"], "evidence": node["evidence"]}
    return {"schema": "plumb.source-acceptance-receipt/v1",
            "intent": copy.deepcopy(value), "checks": checks}


def verify(receipt, value, contract, inventory, observation):
    object_fields(receipt, ("schema", "intent", "checks"))
    expected = collect(value, contract, inventory, observation)
    if receipt != expected:
        raise Refusal("source acceptance receipt differs from independently verified evidence")
    return fingerprint(receipt)
