import re

from .acceptance import commit, declaration, intent
from .blob import Refusal, fingerprint, object_fields, sha
from .execution import Execution
from .source import resolve


PLATFORMS = {
    "linux": ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
    "windows": ("windows-2025", "x86_64-pc-windows-msvc"),
    "macos": ("macos-15", "aarch64-apple-darwin"),
}
PATHS = [".cargo", ".github", "Cargo.toml", "Cargo.lock", "crates", "apps", "packages",
         "plumb.toml", "ectropy.toml", "AGENTS.md", "rust-toolchain.toml", "rustfmt.toml",
         "clippy.toml", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "tsconfig.json",
         ".gitignore", ".npmrc", ".plumb", "Containerfile", "LICENSE", "biome.json", "charts",
         "locus.toml", "runseal.toml", "sidecar.toml", "skills"]


def request(value):
    object_fields(value, ("schema", "source", "configuration", "toolchains"))
    if value["schema"] != "plumb.source-suite-request/v1":
        raise Refusal("unsupported source suite request")
    commit(value["source"])
    sha(value["configuration"])
    if not isinstance(value["toolchains"], dict) or set(value["toolchains"]) != set(PLATFORMS):
        raise Refusal("source suite requires all three native tool worlds")
    for world in value["toolchains"].values():
        object_fields(world, ("channel", "rustc", "cargo"))
        if not isinstance(world["channel"], str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", world["channel"]):
            raise Refusal("source suite requires a pinned Rust toolchain")
        for tool in ("rustc", "cargo"):
            if not isinstance(world[tool], str) or not world[tool].startswith(tool + " " + world["channel"] + " "):
                raise Refusal("source suite requires exact tool version output")
    return value


def graph(value):
    request(value)
    nodes = []
    for platform, (_, target) in PLATFORMS.items():
        payload = {"schema": "plumb.source-suite-work/v1", "source": value["source"],
                   "configuration": value["configuration"], "platform": platform,
                   "target": target, "toolchain": value["toolchains"][platform]}
        nodes.append({"id": platform, "inputs": {
            "source": {"tree": {"paths": PATHS, "source": "product"}},
            "implementation": {"tree": {"paths": [".github/scripts"], "source": "control"}},
            "configuration": value["configuration"], "contract": {"value": payload}},
            "outputs": ["binary"], "evidence": ["receipt"],
            "execution": {"entry": "suite.native", "capability": platform,
                          "materialize": ["source"], "payload": payload}})
    return {"schema": "plumb.blob-graph/v1", "nodes": nodes, "targets": list(PLATFORMS)}


def prepare(value, context, sources, inventory, origin):
    raw = graph(value)
    resolved = resolve(raw, sources)
    contract = {"schema": "plumb.source-suite/v1", "source": value["source"],
                "configuration": value["configuration"], "control": context["control"],
                "graph": resolved, "checks": {name: name for name in PLATFORMS},
                "jobs": {name: "source-suite (" + name + ")" for name in PLATFORMS}}
    selected = intent(value["source"], value["configuration"], fingerprint(contract), context)
    declared = declaration(selected, contract)
    execution = Execution(declared, inventory, origin)
    work = {name: execution.prepare(name) for name in PLATFORMS}
    return {"schema": "plumb.source-suite-plan/v1", "intent": selected,
            "contract": contract, "request": value, "work": work}


def validate(bundle):
    object_fields(bundle, ("schema", "intent", "contract", "request", "work"))
    if bundle["schema"] != "plumb.source-suite-plan/v1":
        raise Refusal("unsupported source suite plan")
    value = request(bundle["request"])
    selected = bundle["intent"]
    if value["source"] != selected["source"] or value["configuration"] != selected["configuration"]:
        raise Refusal("suite plan changed its requested identity")
    declaration(selected, bundle["contract"])
    if not isinstance(bundle["work"], dict) or set(bundle["work"]) != set(PLATFORMS):
        raise Refusal("suite plan omits a native platform")
    return bundle
