use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("plumb should run")
}

#[test]
fn ledger() {
    let output = run(&["cookbook"]);
    assert!(output.status.success());
    let held = String::from_utf8_lossy(&output.stdout).to_string();
    for name in ["seat", "wayfinder"] {
        assert!(held.contains(name), "{held}");
    }
    let exits = held.lines().filter(|line| line.contains("EXIT:")).count();
    let names = held.lines().filter(|line| !line.contains("EXIT:")).count();
    assert_eq!(exits, names, "{held}");
}

#[test]
fn entry() {
    let output = run(&["cookbook", "seat"]);
    assert!(output.status.success());
    let held = String::from_utf8_lossy(&output.stdout).to_string();
    for part in ["# seat", "## Trigger", "## Move", "## Evidence", "## EXIT"] {
        assert!(held.contains(part), "{held}");
    }
}

#[test]
fn unknown() {
    let output = run(&["cookbook", "nosuch"]);
    assert!(!output.status.success());
    let held = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(held.contains("unknown cookbook entry"), "{held}");
    assert!(held.contains("seat, wayfinder"), "{held}");
}
