mod fixture;
use fixture::{fixture, git};
mod status;

use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    let home = super::configuration::support::home();
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(arguments)
        .env("PLUMB_HOME", home.path())
        .env("CLOUDFLARE_API_TOKEN", "private-provider-value")
        .output()
        .unwrap()
}

#[test]
fn surface() {
    let output = run(&["lane", "--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for command in ["status", "inspect", "plan", "apply", "discard", "result"] {
        assert!(text.contains(command), "{text}");
    }
    for command in ["dispose", "build", "publish", "repair"] {
        assert!(!run(&["lane", command, "--help"]).status.success());
    }
    assert!(!run(&["lane", "status", "--json"]).status.success());
    let output = run(&["lane", "inspect", "--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for flag in [
        "--repository",
        "--app",
        "--lane",
        "--origin",
        "--authority",
        "--digest",
        "--request",
        "--revision",
    ] {
        assert!(text.contains(flag), "{text}");
    }
}
