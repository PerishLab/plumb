use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .args(args)
        .output()
        .expect("plumb should run")
}

#[test]
fn ledger() {
    let output = run(&["cookbook"]);
    assert!(output.status.success());
    let held = String::from_utf8_lossy(&output.stdout).to_string();
    for code in [
        "guard.integration-branch",
        "structure.seat-member",
        "structure.known-directory",
    ] {
        assert!(held.contains(code), "{held}");
    }
    let exits = held.lines().filter(|line| line.contains("EXIT:")).count();
    let codes = held.lines().filter(|line| !line.contains("EXIT:")).count();
    assert_eq!(exits, codes, "{held}");
}

#[test]
fn entry() {
    let output = run(&["cookbook", "structure.seat-member"]);
    assert!(output.status.success());
    let held = String::from_utf8_lossy(&output.stdout).to_string();
    for part in [
        "# structure.seat-member",
        "## Trigger",
        "## Solution",
        "## Evidence",
        "## EXIT",
    ] {
        assert!(held.contains(part), "{held}");
    }
}

#[test]
fn json() {
    let output = run(&["cookbook", "structure.seat-member", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["code"], "structure.seat-member");
    assert!(value["trigger"].is_string());
    assert!(value["solution"].is_string());

    let output = run(&["cookbook", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["entries"].as_array().expect("entries").len(), 7);
}

#[test]
fn unknown() {
    let output = run(&["cookbook", "nosuch"]);
    assert!(!output.status.success());
    let held = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(held.contains("unknown Cookbook code"), "{held}");
    assert!(held.contains("structure.seat-member"), "{held}");
}
