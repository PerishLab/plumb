use super::stable::{command, repo};
#[test]
fn retraction() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    repo(root, "https://forge.test/PerishLab/probe.git");

    let exact = command(root, &["stable", "retract", "--version", "v0.10.2-beta.1"]);
    let refused = String::from_utf8_lossy(&exact.stderr).to_string();
    assert!(
        refused.contains("does not belong to channel stable"),
        "{refused}"
    );

    let plan = command(
        root,
        &["stable", "retract", "--version", "0.10.2", "--dry-run"],
    );
    let printed = String::from_utf8_lossy(&plan.stdout).to_string();
    let refusal = String::from_utf8_lossy(&plan.stderr).to_string();
    assert!(
        !plan.status.success(),
        "a preview that cannot read the authority must refuse: {printed}"
    );
    assert!(
        refusal.contains("https://releases.test/v1/releases/stable/v0.10.2/seal.json"),
        "a refusal must name what it could not read: {refusal}"
    );
    assert!(
        !printed.contains("--delete"),
        "a refused preview must not have printed a plan: {printed}"
    );
}
