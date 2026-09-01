use std::path::Path;

fn canonical() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/workflows/ship.yml"))
        .expect("Plumb owns one canonical ship workflow")
}

fn bootstrap() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/scripts/bootstrap-plumb.sh"))
        .expect("Plumb owns one canonical bootstrap")
}

fn windows() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/scripts/bootstrap-plumb.ps1"))
        .expect("Plumb owns one canonical Windows bootstrap")
}

fn executor() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/scripts/execute-ship.sh"))
        .expect("Plumb owns one canonical ship executor")
}

fn transport() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
        .expect("Plumb owns binary source discovery")
}

#[test]
fn matrix() {
    let held = canonical();
    assert!(held.contains("workflow_dispatch:"), "{held}");
    assert!(held.contains(".forgejo/scripts/resolve-ship.sh"), "{held}");
    assert!(
        held.contains("timeout --kill-after=5s 45s"),
        "marker checkout must not inherit an unbounded transport wait"
    );
    assert!(
        held.contains("fromJSON(needs.resolve.outputs.workload)"),
        "{held}"
    );
    assert!(
        held.contains("fromJSON(needs.publish_plan.outputs.publication)"),
        "{held}"
    );
    assert!(executor().contains("plumb ship execute --request"));
    assert!(held.contains("\n  workload:\n"), "{held}");
    assert!(held.contains("\n  publish_plan:\n"), "{held}");
    assert!(held.contains("\n  publication:\n"), "{held}");
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
    assert!(
        !held.contains("cargo build --quiet --locked --manifest-path"),
        "workflow orchestration must not duplicate atom bootstrap implementation"
    );
    assert!(!held.contains("${{ runner.temp }}"), "{held}");
    assert!(
        !held.contains("PLUMB_HOME:"),
        "ship does not own a depot seat"
    );
    assert!(
        !held.contains("corepack enable"),
        "request owns preparation"
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
fn depot() {
    let held = bootstrap();
    let windows = windows();
    let installed = held
        .find("cp \"$atom/target/debug/plumb\" \"$tool\"")
        .expect("exact Plumb install");
    let bootstrap = held
        .find("\"$tool\" configuration install")
        .expect("bootstrap configuration");
    let exact = held
        .rfind("\"$tool\" configuration install")
        .expect("exact configuration");
    assert_eq!(
        held.matches("\"$tool\" configuration install").count(),
        2,
        "each bootstrap phase should own one sync branch"
    );
    assert!(
        bootstrap < installed && installed < exact,
        "bootstrap rules must precede the build while exact rules follow installation"
    );
    for binding in [
        "PLUMB_BUILD_VERSION=\"$PLUMB_RELEASE_VERSION\"",
        "PLUMB_BUILD_CHANNEL=\"$PLUMB_RELEASE_CHANNEL\"",
        "PLUMB_BUILD_COMMIT=\"$PLUMB_RELEASE_COMMIT\"",
    ] {
        assert!(held.contains(binding), "atom build omits {binding}");
    }
    for binding in [
        "$env:PLUMB_BUILD_VERSION = $env:PLUMB_RELEASE_VERSION",
        "$env:PLUMB_BUILD_CHANNEL = $env:PLUMB_RELEASE_CHANNEL",
        "$env:PLUMB_BUILD_COMMIT = $env:PLUMB_RELEASE_COMMIT",
    ] {
        assert!(
            windows.contains(binding),
            "Windows atom build omits {binding}"
        );
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
    assert!(held.contains(":(glob)**/Cargo.toml"), "{held}");
    assert!(held.contains("seat.join(\"src\")"), "{held}");
    assert!(held.contains("spec.depends.get(\"binary\")"), "{held}");
    assert!(!held.contains("seat.join(\"tests\")"), "{held}");
}
