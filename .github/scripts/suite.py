import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from execute import Executor
from lib.acceptance import WORKFLOW, commit
from lib.blob import Refusal, decode, digest, encode, fingerprint
from lib.collection import receive
from lib.handoff import pack, regular
from lib.inputs import Inputs
from lib.inventory import Inventory
from lib.r2 import R2
from lib.source import Sources, resolve
from lib.suite import PLATFORMS, graph, prepare, validate


def context():
    return {"repository": os.environ["GITHUB_REPOSITORY"], "control": os.environ["GITHUB_SHA"],
            "workflow": WORKFLOW, "run": int(os.environ["GITHUB_RUN_ID"]),
            "attempt": int(os.environ["GITHUB_RUN_ATTEMPT"])}


def checkout(root, expected):
    actual = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], timeout=30).decode().strip()
    if actual != commit(expected):
        raise Refusal("suite checkout differs from its exact selected commit")


def load(path, expected):
    body = regular(path)
    if digest(body) != expected:
        raise Refusal("suite plan differs from its trusted planner output")
    bundle = validate(decode(body))
    if bundle["intent"]["context"] != context():
        raise Refusal("suite plan belongs to another workflow execution")
    return bundle


def emit(name, value):
    value = str(value)
    if "\n" in value or "\r" in value:
        raise Refusal("workflow output must be one line")
    with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
        output.write(name + "=" + value + "\n")


def plan(args):
    inventory = Inventory(R2.environment())
    value = decode(inventory.download(os.environ["PLUMB_SUITE_DECLARATION"]))
    if value.get("source") != os.environ["PLUMB_SUITE_SOURCE"]:
        raise Refusal("source suite request differs from its explicitly selected source")
    checkout(args.root, value["source"])
    checkout(args.control, context()["control"])
    inventory.verify(value["configuration"])
    bundle = prepare(value, context(), Sources({"product": args.root, "control": args.control}),
                     inventory, os.environ["PLUMB_WORKFLOW_INVENTORY_URL"])
    args.output.mkdir(parents=True, exist_ok=False)
    body = encode(bundle)
    (args.output / "plan.json").write_bytes(body)
    emit("digest", digest(body))
    emit("matrix", encode({"include": [{"platform": name, "runner": runner}
                                        for name, (runner, _) in PLATFORMS.items()]}).decode())


def work(args):
    bundle = load(args.plan, args.digest)
    checkout(args.root, bundle["intent"]["source"])
    checkout(args.control, bundle["intent"]["context"]["control"])
    sources = Sources({"product": args.root, "control": args.control})
    raw = graph(bundle["request"])
    if resolve(raw, sources) != bundle["contract"]["graph"]:
        raise Refusal("candidate materialization differs from the trusted plan")
    prepared = bundle["work"][args.platform]
    if prepared["state"] == "reuse":
        args.output.mkdir(parents=True, exist_ok=False)
        (args.output / "reuse.json").write_bytes(encode({"key": prepared["completion"]["key"]}))
        return
    if prepared["state"] != "run":
        raise Refusal("suite plan has unresolved native work")
    seat = Path(tempfile.mkdtemp(prefix="suite-work-", dir=os.environ.get("RUNNER_TEMP")))
    executor = Executor(Path(__file__).with_name("suite_worker.py"), args.root, 8100,
                        Inputs.select(raw, args.platform, sources))
    result = executor(prepared["request"], seat)
    manifest = pack(result, args.output)
    (args.output / "handoff.json").write_bytes(encode(manifest))


def collect(args):
    bundle = load(args.plan, args.digest)
    checkout(args.control, bundle["intent"]["context"]["control"])
    inventory = Inventory(R2.environment())
    original = decode(inventory.download(os.environ["PLUMB_SUITE_DECLARATION"]))
    if original != bundle["request"] or original["source"] != os.environ["PLUMB_SUITE_SOURCE"]:
        raise Refusal("collected suite differs from the dispatched request")
    artifacts = {}
    for platform in PLATFORMS:
        root = args.artifacts / ("source-suite-" + platform + "-" + str(context()["attempt"]))
        if bundle["work"][platform]["state"] == "run":
            artifacts[platform] = (decode(regular(root / "handoff.json")), root)
        elif not (root / "reuse.json").is_file():
            raise Refusal("cached native suite has no completed job handoff")
    receipt = receive(bundle["intent"], bundle["contract"], artifacts, inventory)
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / "receipt.json").write_bytes(encode(receipt))
    emit("receipt", inventory.upload(encode(receipt)))
    emit("contract", fingerprint(bundle["contract"]))


def main():
    parser = argparse.ArgumentParser(description="Plan, execute, or independently collect a source-only suite")
    parser.add_argument("operation", choices=("plan", "work", "collect"))
    parser.add_argument("--root", type=Path, default=Path("candidate"))
    parser.add_argument("--control", type=Path, default=Path("control"))
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--plan", type=Path)
    parser.add_argument("--digest")
    parser.add_argument("--platform", choices=tuple(PLATFORMS))
    parser.add_argument("--artifacts", type=Path)
    args = parser.parse_args()
    if args.operation != "plan" and (args.plan is None or not args.digest):
        parser.error("work and collect require the exact trusted plan and digest")
    if args.operation == "work" and args.platform is None:
        parser.error("work requires a native platform")
    if args.operation == "collect" and args.artifacts is None:
        parser.error("collect requires downloaded artifact roots")
    {"plan": plan, "work": work, "collect": collect}[args.operation](args)


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, KeyError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
