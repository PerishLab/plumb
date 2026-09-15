use std::path::Path;

#[path = "../../../src/command/ship/transport/reuse.rs"]
mod ship;

#[test]
fn proof() {
    for reason in ["proof-held", "publication-moved"] {
        let node = serde_json::json!({
            "reason": reason,
            "reuse": {"type": "workload", "source": "https://example.invalid/held.tgz"}
        });
        assert!(ship::workload(&node));
    }
    for reason in ["proof-moved", "record-absent", "unknown"] {
        let node = serde_json::json!({
            "decision": "run", "reason": reason,
            "reuse": {"type": "workload", "source": "https://example.invalid/held.tgz"}
        });
        assert!(
            !ship::workload(&node),
            "{reason} must not skip its workload"
        );
    }
    assert!(!ship::workload(&serde_json::json!({})));
}

fn canonical() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.forgejo/workflows/ship.yml");
    std::fs::read_to_string(path).expect("Plumb owns one canonical ship workflow")
}

#[test]
fn image() {
    let held = canonical();
    let (_, image) = held.split_once("    container:\n      image: ").unwrap();
    let image = image.lines().next().unwrap();
    assert!(image.contains("@sha256:"));
    assert_eq!(
        held.matches(&format!("    container:\n      image: {image}\n"))
            .count(),
        3
    );
    let workload = held.split_once("  workload:\n").unwrap().1;
    let workload = workload.split_once("  publish_plan:\n").unwrap().0;
    assert!(workload.contains(&format!(
        "    runs-on: ${{{{ matrix.runner }}}}\n    container:\n      image: ${{{{ matrix.runner == 'linux' && '{image}' || '' }}}}\n"
    )));
}

fn transport() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
        .expect("Plumb owns binary source discovery")
}

fn resolver() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ["resolve", "production"]
        .iter()
        .map(|name| {
            std::fs::read_to_string(
                root.join(format!("crates/cli/src/command/ship/transport/{name}.rs")),
            )
            .expect("Plumb owns ship graph resolution")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolution() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/scripts/resolve-ship.sh"))
        .expect("Plumb owns ship output projection")
}

fn marker() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/scripts/fetch-marker.sh"))
        .expect("Plumb owns marker transport")
}

#[test]
fn matrix() {
    let held = canonical();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let unix = std::fs::read_to_string(root.join(".forgejo/scripts/bootstrap-plumb.sh"))
        .expect("Plumb owns the Unix atom bootstrap");
    let windows = std::fs::read_to_string(root.join(".forgejo/scripts/bootstrap-plumb.ps1"))
        .expect("Plumb owns the Windows atom bootstrap");
    assert!(
        unix.contains("cargo --config \"$atom/.cargo/config.toml\" build"),
        "Unix cold builds must carry the exact atom Cargo context: {unix}"
    );
    assert!(
        windows.contains("cargo --config (Join-Path $atom '.cargo/config.toml') build"),
        "Windows cold builds must carry the exact atom Cargo context: {windows}"
    );
    assert!(held.contains("workflow_dispatch:"), "{held}");
    assert!(held.contains("PLUMB_BUILD_VERSION: ${{ inputs.plumb }}"));
    assert!(held.contains("PLUMB_BUILD_COMMIT: ${{ github.sha }}"));
    assert!(held.contains(".forgejo/scripts/resolve-ship.sh"), "{held}");
    assert!(
        held.contains(".forgejo/scripts/fetch-marker.sh"),
        "marker checkout must use the bounded canonical transport"
    );
    assert!(
        held.contains(".forgejo/scripts/fetch-marker.ps1"),
        "Windows marker checkout must use the bounded canonical transport"
    );
    assert!(held.contains("runner.os != 'Windows'"), "{held}");
    assert!(held.contains("runner.os == 'Windows'"), "{held}");
    let marker = marker();
    assert!(marker.contains("bounded_fetch()"), "{marker}");
    assert!(marker.contains("kill -TERM \"$fetch_pid\""), "{marker}");
    assert!(marker.contains("kill -KILL \"$fetch_pid\""), "{marker}");
    assert!(
        !marker.contains("timeout --"),
        "marker transport must not require GNU coreutils on macOS: {marker}"
    );
    assert!(held.contains("fromJSON(needs.resolve.outputs.workload)"));
    assert!(
        held.contains("fromJSON(needs.publish_plan.outputs.publication)"),
        "{held}"
    );
    assert!(
        held.contains("repository: ${{ inputs.repository }}\n          ref: ${{ inputs.marker }}"),
        "publication planning must retain the marker ref for exact verification"
    );
    for foreign in [
        "\n  seal:\n",
        "\n  configuration:\n",
        "\n  verify:\n",
        "\n  depot:\n",
    ] {
        assert!(
            !held.contains(foreign),
            "ship retains foreign phase {foreign}"
        );
    }
    assert!(
        !held.contains("plumb depot "),
        "ship must not operate depot"
    );
    assert!(
        held.contains(".forgejo/scripts/bootstrap-plumb.sh"),
        "the canonical workflow should consume its Plumb-owned bootstrap script"
    );
    assert!(
        held.contains(".forgejo/scripts/bootstrap-plumb.ps1"),
        "the canonical workflow should consume its Plumb-owned Windows bootstrap script"
    );
    assert_eq!(
        held.matches("bootstrap-plumb.sh .plumb-atom exact").count(),
        4,
        "every Unix job must install the exact atom configuration"
    );
    assert_eq!(
        held.matches("bootstrap-plumb.ps1 .plumb-atom exact")
            .count(),
        1,
        "the Windows workload must install the exact atom configuration"
    );
    assert!(
        !held.contains("cargo build --quiet --locked --manifest-path"),
        "workflow orchestration must not duplicate atom bootstrap implementation"
    );
    assert!(
        !held.contains("name: release-${{ matrix.target }}"),
        "binary workloads must not cross Forgejo artifact storage"
    );
    assert_eq!(held.matches(".forgejo/scripts/execute-ship").count(), 3);
    assert!(
        held.contains("resolve-ship.sh '${{ github.sha }}' ready"),
        "publication planning must observe the recorded workloads"
    );
    for binding in [
        "if: matrix.control != 'reuse' && (matrix.request.operation.type != 'bind' || matrix.request.operation.build.reuse.type != 'workload')",
        "needs.resolve.outputs.publication_missing == 'true'",
        "PLUMB_RELEASE_VERSION: ${{ needs.publish_plan.outputs.version }}",
        "atom_handoff: ${{ steps.plan.outputs.atom_handoff }}",
        "PLUMB_RELEASE_COMMIT: ${{ needs.publish_plan.outputs.commit }}",
        "if: needs.resolve.outputs.workload_missing == 'true'",
        "needs.workload.result == 'skipped'",
        "needs.publish_plan.result == 'success'",
    ] {
        assert!(held.contains(binding), "ship carry omits {binding}");
    }
    let handoff = "PLUMB_ATOM_HANDOFF: ${{ needs.resolve.outputs.atom_handoff }}";
    assert_eq!(
        held.matches(handoff).count(),
        2,
        "Linux-only publication handoff wiring"
    );
    assert!(held.contains(
        "PLUMB_ATOM_HANDOFF: ${{ runner.os == 'Linux' && needs.resolve.outputs.atom_handoff || '' }}"
    ));
    let windows = held
        .split_once("Install the exact atom Plumb on Windows")
        .unwrap()
        .1;
    assert!(
        !windows
            .split_once("Add the requested Rust target")
            .unwrap()
            .0
            .contains(handoff)
    );
    let resolve = held.split_once("  workload:\n").unwrap().0;
    assert!(!resolve.contains(handoff), "resolve cannot consume itself");
    assert!(
        resolution().contains("publication_ready=$(printf '%s' \"$graph\""),
        "resolve must expose whether its publication plan is already final"
    );
    assert!(resolution().contains("{type:\"workload\",source:$source}"));
    let graph = resolver();
    assert!(
        graph.contains("!pending.is_empty() || (self.spec.binary() && self.workload.missing)"),
        "cold binary workloads must retain publication intent for publish_plan"
    );
    assert!(
        graph.contains("workflow::plan::derive("),
        "ship must resolve only the requested action instead of probing every unrelated action: {graph}"
    );
    assert!(
        graph.contains("Some(action)"),
        "ship must select one action instead of deriving the whole workflow graph: {graph}"
    );
    assert!(graph.contains("action == \"ship/oci\""), "{graph}");
    assert!(
        graph.contains("carry(&mut request, &self.workload.reuse)"),
        "OCI must consume the binary workloads without waiting for a release seal"
    );
    assert!(
        !graph.contains("format!(\"atom={}"),
        "an orchestration atom must not invalidate an immutable publication"
    );
    assert!(
        !graph.contains("format!(\"plumb={}"),
        "a Plumb patch must not invalidate an immutable publication"
    );
    assert!(
        graph.contains("super::seal::held(self.marker)?"),
        "an existing marker seal must hold binary publication across atom changes"
    );
}

#[test]
fn opaque() {
    let held = canonical();
    for leaked in [
        "matrix.medium",
        "plumb ship npm",
        "plumb ship chart",
        "plumb ship oci",
        "plumb ship cargo",
        "plumb ship cfworker",
        "plumb workflow plan",
    ] {
        assert!(!held.contains(leaked), "workflow leaks {leaked}: {held}");
    }
}

#[test]
fn singular() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let templates = root.join("assets/ship");
    assert!(
        std::fs::read_dir(&templates)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true),
        "canonical workflow must have no template double"
    );
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
        .arg("--help")
        .output()
        .expect("plumb help");
    assert!(help.status.success());
    assert!(
        !String::from_utf8_lossy(&help.stdout)
            .lines()
            .any(|line| line.trim_start().starts_with("lane ")),
        "downstream workflow rendering must not remain public"
    );
}

#[test]
fn inputs() {
    let held = transport();
    assert!(held.contains("super::sources::read(&spec.root)?"), "{held}");
    assert!(held.contains("spec.depends.get(\"binary\")"), "{held}");
    assert!(
        held.contains("Cargo.toml#/workspace/package/version"),
        "{held}"
    );
    assert!(!held.contains("package.json#/version"), "{held}");
    assert!(!held.contains("seat.join(\"tests\")"), "{held}");
}
