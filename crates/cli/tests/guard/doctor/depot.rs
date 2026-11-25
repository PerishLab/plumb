use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn doctor(home: &Path, root: &Path, mark: Option<&str>) -> (Output, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(["doctor", "--json", root.to_str().expect("utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env_remove("PLUMB_DEPOT_MARK")
        .env("PLUMB_HOME", home);
    if let Some(mark) = mark {
        command.env("PLUMB_DEPOT_MARK", mark);
    }
    let output = command.output().expect("plumb doctor should run");
    let report = serde_json::from_slice(&output.stdout).expect("doctor json");
    (output, report)
}

fn inspect(home: &Path, root: &Path, log: &Path) -> (String, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["cache", "transport", "--json", "--diagnostic"])
        .arg(log)
        .arg(root)
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home)
        .output()
        .expect("plumb cache transport should run");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("transport json");
    let line = std::fs::read_to_string(log)
        .expect("diagnostic log")
        .lines()
        .next()
        .expect("start event")
        .to_owned();
    let event: Value = serde_json::from_str(&line).expect("start json");
    let mark = event["data"]["depot"]
        .as_str()
        .expect("depot mark")
        .to_owned();
    (mark, report)
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .status()
        .expect("git should run");
    assert!(status.success());
}

#[test]
fn hookless() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("plumb.toml"), "[layout]\n").expect("governance");
    std::fs::remove_file(root.join(".git/hooks/pre-commit")).expect("remove projected hook");
    let held = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(held.contains("pre-commit is absent"), "{held}");
    assert!(held.contains("run plumb configuration install"), "{held}");
}

#[test]
fn pin() {
    let fixture = super::fixture();
    let root = fixture.path();
    git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/Example/probe.git",
        ],
    );
    git(root, &["add", "plumb.toml"]);
    let home = tempfile::tempdir().expect("home");
    let (mark, _) = inspect(home.path(), root, &home.path().join("base.jsonl"));

    let (output, report) = doctor(home.path(), root, None);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["ok"], true);

    let (output, report) = doctor(home.path(), root, Some(&mark));
    assert!(output.status.success(), "{report}");
    assert_eq!(report["ok"], true);

    let wrong = "0".repeat(64);
    let (output, report) = doctor(home.path(), root, Some(&wrong));
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["ok"], false);
    assert!(
        report["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| {
                finding["code"] == "config.plumb-current-and-guarded"
                    && finding["grade"] == "out of true"
            }),
        "{report}"
    );

    let upper = "A".repeat(64);
    for bad in ["", "   ", "not-a-mark", upper.as_str()] {
        let (output, report) = doctor(home.path(), root, Some(bad));
        assert!(!output.status.success(), "{bad:?}: {report}");
        assert_eq!(report["ok"], false, "{bad:?}: {report}");
        assert!(
            report["findings"]
                .as_array()
                .expect("findings")
                .iter()
                .any(|finding| {
                    finding["code"] == "config.plumb-current-and-guarded"
                        && finding["grade"] == "blind"
                }),
            "{bad:?}: {report}"
        );
    }

    std::fs::create_dir_all(home.path().join("overlay")).expect("overlay dir");
    std::fs::write(home.path().join("overlay/probe.txt"), "overlay\n").expect("overlay file");
    let (next, _) = inspect(home.path(), root, &home.path().join("next.jsonl"));
    assert_ne!(mark, next);

    let (output, report) = doctor(home.path(), root, Some(&mark));
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["ok"], false);

    let (output, report) = doctor(home.path(), root, Some(&next));
    assert!(output.status.success(), "{report}");
    assert_eq!(report["ok"], true);
}

#[test]
fn transport() {
    let fixture = super::fixture();
    let root = fixture.path();
    git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/Example/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("source");
    let status = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .status()
        .expect("cargo lockfile");
    assert!(status.success());
    git(root, &["add", "."]);

    let home = tempfile::tempdir().expect("home");
    let (first, before) = inspect(home.path(), root, &home.path().join("first.jsonl"));
    std::fs::create_dir_all(home.path().join("overlay")).expect("overlay dir");
    std::fs::write(home.path().join("overlay/probe.txt"), "overlay\n").expect("overlay file");
    let (next, after) = inspect(home.path(), root, &home.path().join("next.jsonl"));

    assert_ne!(first, next);
    assert_eq!(before["restore"], after["restore"]);
    assert_eq!(before["key"], after["key"]);
}
