use std::path::Path;

fn canonical() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(".forgejo/workflows/ship.yml"))
        .expect("Plumb owns one canonical ship workflow")
}

#[test]
fn matrix() {
    let held = canonical();
    assert!(held.contains("workflow_dispatch:"), "{held}");
    assert!(held.contains("graph=$(plumb ship resolve"), "{held}");
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
    assert!(!held.contains("${{ runner.temp }}"), "{held}");
    assert_eq!(held.matches("PLUMB_HOME: /tmp/plumb-home").count(), 2);
    assert!(
        held.contains("matrix.operation.type == 'cfworker'"),
        "{held}"
    );
    assert!(held.contains("matrix.operation.type == 'npm'"), "{held}");
    assert!(held.contains("run: corepack enable"), "{held}");
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
