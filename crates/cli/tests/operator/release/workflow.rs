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

fn transport() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
        .expect("Plumb owns binary source discovery")
}

#[test]
fn matrix() {
    let held = canonical();
    assert!(held.contains("workflow_dispatch:"), "{held}");
    assert!(held.contains("graph=$(plumb ship resolve"), "{held}");
    assert!(
        held.contains("timeout --kill-after=5s 45s"),
        "marker checkout must not inherit an unbounded transport wait"
    );
    assert!(
        held.contains("fromJSON(needs.resolve.outputs.targets)"),
        "{held}"
    );
    assert!(
        held.contains("fromJSON(needs.resolve.outputs.project)"),
        "{held}"
    );
    assert!(held.contains("plumb ship execute --request"), "{held}");
    assert!(held.contains("\n  materialize:\n"), "{held}");
    assert!(held.contains("\n  seal:\n"), "{held}");
    assert!(held.contains("\n  configuration:\n"), "{held}");
    assert!(held.contains("\n  verify:\n"), "{held}");
    assert!(held.contains("\n  depot:\n"), "{held}");
    let immutable = held
        .rfind("plumb ship execute --request")
        .expect("ship project");
    let consensus = held
        .rfind("plumb depot channel --marker")
        .expect("depot consensus");
    let configuration = held
        .rfind("plumb depot configuration --marker")
        .expect("configuration derivative");
    let skill = held
        .rfind("plumb depot skill --marker")
        .expect("skill derivative");
    assert!(
        configuration < immutable && immutable < consensus && skill < consensus,
        "exact configuration must precede manager readback while every latest pointer stays last"
    );
    assert!(
        held.contains("needs: [resolve, seal, configuration]"),
        "manager verification must wait for marker-exact configuration"
    );
    assert_eq!(
        held.matches("plumb depot channel --marker").count(),
        1,
        "only the final depot job may move the channel"
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
    assert_eq!(held.matches("PLUMB_HOME: /tmp/plumb-home").count(), 2);
    assert_eq!(held.matches("run: corepack enable").count(), 1, "{held}");
    assert!(
        !held.contains("name: release-${{ matrix.target }}"),
        "binary workloads must not cross Forgejo artifact storage"
    );
    assert_eq!(
        held.matches("plumb workflow record $env:PLUMB_BINARY_ACTION")
            .count()
            + held
                .matches("plumb workflow record \"$PLUMB_BINARY_ACTION\"")
                .count(),
        2,
        "each native runner dialect must record its workload directly"
    );
    assert!(
        held.contains("- name: Resolve and fetch every binary workload"),
        "seal must consume the recorded workload URLs"
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
    let bootstrap = held.find("\"$tool\" depot sync").expect("bootstrap sync");
    let exact = held.rfind("\"$tool\" depot sync").expect("exact sync");
    assert_eq!(
        held.matches("\"$tool\" depot sync").count(),
        2,
        "each bootstrap phase should own one sync branch"
    );
    assert!(
        bootstrap < installed && installed < exact,
        "bootstrap rules must precede the build while exact rules follow installation"
    );
    assert!(
        canonical().contains("bootstrap-plumb.sh .plumb-atom exact"),
        "the depot consumer must select exact configuration"
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
