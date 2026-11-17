use std::process::Command;

#[test]
fn help() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["instance", "--json", "--help"])
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not proof"));
}

#[test]
fn refusal() {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["instance", "--path", "fixture.pub"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
}
