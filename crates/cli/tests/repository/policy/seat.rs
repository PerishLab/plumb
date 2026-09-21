use std::path::Path;
use std::process::Command;

fn estate() -> Vec<(String, String)> {
    crate::support::rules(&["policy.toml", "deps.toml", "structure.toml"])
}

pub(super) fn run(root: &Path) -> String {
    let home = root.join(".plumb-test-home");
    let held = estate();
    crate::support::stock(
        &home.join("configurations"),
        &held
            .iter()
            .map(|(path, body)| (path.as_str(), body.as_str()))
            .collect::<Vec<_>>(),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env("PLUMB_HOME", home)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub(super) fn policy(root: &Path, write: bool) -> std::process::Output {
    let home = root.join(".plumb-test-home");
    let held = estate();
    crate::support::stock(
        &home.join("configurations"),
        &held
            .iter()
            .map(|(path, body)| (path.as_str(), body.as_str()))
            .collect::<Vec<_>>(),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command.args(["policy", root.to_str().expect("path should be utf8")]);
    command.env("PLUMB_HOME", home);
    if write {
        command.arg("--write");
    }
    command.output().expect("plumb should run")
}
