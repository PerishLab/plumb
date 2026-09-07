use super::stable::{command, repo};

#[test]
fn retired() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    repo(root, "https://forge.test/PerishLab/probe.git");
    for version in ["v0.10.2-beta.1", "v0.10.2"] {
        let output = command(root, &["release", "retract", "--version", version]);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("unrecognized subcommand 'retract'"),
            "{error}"
        );
        assert!(output.stdout.is_empty());
    }
    let help = command(root, &["release", "--help"]);
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("Withdraw"));
}
