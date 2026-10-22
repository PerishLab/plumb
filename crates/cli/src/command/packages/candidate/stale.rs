use super::super::{Seat, State, engine, work};
use super::Fixture;
use serde_json::json;
use std::path::PathBuf;

fn lock(version: &str) -> String {
    format!("lockfileVersion: '9.0'\npackages:\n  '@perishlab/probe@{version}': {{}}\n")
}

fn isolated(name: &str) -> bool {
    if std::env::var("PLUMB_TEST_STALE").as_deref() == Ok(name) {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    std::fs::write(tools.path().join("latest"), "\"0.1.0\"").unwrap();
    std::fs::write(tools.path().join("lock"), lock("0.1.0")).unwrap();
    super::writer::executable(
        &tools.path().join("pnpm"),
        r#"#!/bin/sh
seat=$(dirname "$0")
if [ "$1" = --config.ignore-pnpmfile=true ]; then shift; fi
case "$1" in
config) echo null ;;
view) cat "$seat/latest" ;;
update) cp "$seat/lock" pnpm-lock.yaml ;;
--version) echo 11.28.5 ;;
*) exit 91 ;;
esac
"#,
    );
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            &format!("command::packages::follow::tests::stale::{name}"),
            "--nocapture",
        ])
        .env("PLUMB_TEST_STALE", name)
        .env("PLUMB_STALE_TOOLS", tools.path())
        .env("PLUMB_HOME", home.path())
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env_remove("CI");
    for (key, _) in std::env::vars() {
        if (key.starts_with("CARGO_") || key.starts_with("RUST"))
            && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&key.as_str())
        {
            command.env_remove(key);
        }
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    false
}

struct Probe {
    fixture: Fixture,
    source: PathBuf,
    seat: Seat,
    state: State,
}

fn failed() -> Probe {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { false }\n");
    std::fs::write(
        source.join("package.json"),
        r#"{"name":"fixture","dependencies":{"@perishlab/probe":"0"}}"#,
    )
    .unwrap();
    std::fs::write(source.join("pnpm-lock.yaml"), lock("0.1.0")).unwrap();
    work::git(&source, &["add", "."]).unwrap();
    work::git(&source, &["commit", "-q", "-m", "npm base"]).unwrap();
    work::git(&source, &["push", "-q", "origin", "main"]).unwrap();
    work::git(&source, &["fetch", "origin"]).unwrap();
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let text = std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap();
    std::fs::write(
        state.worktree.join("Cargo.toml"),
        format!("{text}description='owned delta'\n"),
    )
    .unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(&state.worktree, &["commit", "-q", "-m", "owned delta"]).unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("failed with"), "{error}");
    state = seat.read("Example/probe").unwrap();
    let head = state.pushed.as_deref().unwrap();
    std::fs::write(fixture.root.path().join("guard.json"), json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":head,"status":"completed","conclusion":"failure","details_url":"https://github.com/Example/probe/actions/runs/17"}).to_string()).unwrap();
    Probe {
        fixture,
        source,
        seat,
        state,
    }
}

fn release(version: &str) {
    let tools = PathBuf::from(std::env::var_os("PLUMB_STALE_TOOLS").unwrap());
    std::fs::write(tools.join("latest"), json!(version).to_string()).unwrap();
    std::fs::write(tools.join("lock"), lock(version)).unwrap();
}

#[test]
fn newer() {
    if !isolated("newer") {
        return;
    }
    let Probe {
        fixture,
        source,
        seat,
        mut state,
    } = failed();
    let old = state.pushed.clone().unwrap();
    let main = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    release("0.2.0");
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("failed with"), "{error}");
    let head = fixture.provider().remote(17).unwrap().unwrap();
    assert_ne!(head, old);
    assert!(
        work::git(&source, &["show", &format!("{head}:pnpm-lock.yaml")])
            .unwrap()
            .contains("0.2.0")
    );
    assert_eq!(
        work::git(&source, &["rev-parse", "origin/main"]).unwrap(),
        main
    );
    assert_eq!(state.issue, 17);
    assert_eq!(state.pull, 18);
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["endpoint"] == "repos/Example/probe/pulls"
                && !call["payload"].is_null())
            .count(),
        1
    );
}

fn unchanged(probe: &mut Probe, expected: &str) {
    let Probe {
        fixture,
        source,
        seat,
        state,
    } = probe;
    let before = fixture.calls().len();
    let head = work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap();
    let pushed = fixture.provider().remote(17).unwrap();
    let error = engine::advance(source, &fixture.provider(), seat, state).unwrap_err();
    assert!(error.contains(expected), "{error}");
    assert_eq!(
        work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap(),
        head
    );
    assert_eq!(fixture.provider().remote(17).unwrap(), pushed);
    assert!(
        fixture.calls()[before..]
            .iter()
            .all(|call| call["payload"].is_null())
    );
}

#[test]
fn current() {
    if !isolated("current") {
        return;
    }
    let mut probe = failed();
    unchanged(&mut probe, "organization Guard failed");
    assert!(
        work::git(&probe.state.worktree, &["status", "--porcelain"])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn foreign() {
    if !isolated("foreign") {
        return;
    }
    let mut probe = failed();
    release("0.2.0");
    probe.state.pushed = Some(work::git(&probe.source, &["rev-parse", "origin/main"]).unwrap());
    unchanged(&mut probe, "outside recorded publication");
}

#[test]
fn moved() {
    if !isolated("moved") {
        return;
    }
    let mut probe = failed();
    release("0.2.0");
    work::git(
        &probe.state.worktree,
        &["commit", "--allow-empty", "-q", "-m", "foreign move"],
    )
    .unwrap();
    unchanged(&mut probe, "outside recorded recovery state");
}

#[test]
fn unknown() {
    if !isolated("unknown") {
        return;
    }
    let mut probe = failed();
    let tools = PathBuf::from(std::env::var_os("PLUMB_STALE_TOOLS").unwrap());
    std::fs::write(tools.join("latest"), "not-json").unwrap();
    unchanged(&mut probe, "cannot read npm stable");
}

#[test]
fn remote() {
    if !isolated("remote") {
        return;
    }
    let mut probe = failed();
    release("0.2.0");
    let old = probe.state.pushed.as_deref().unwrap();
    std::fs::write(
        probe.fixture.root.path().join("run.json"),
        json!({"head_sha":old,"path":".github/workflows/guard.yml","event":"pull_request","status":"completed","conclusion":"failure"})
            .to_string(),
    )
    .unwrap();
    let tree = work::git(&probe.source, &["rev-parse", &format!("{old}^{{tree}}")]).unwrap();
    let moved = work::git(
        &probe.source,
        &["commit-tree", &tree, "-p", old, "-m", "foreign move"],
    )
    .unwrap();
    work::git(
        &probe.source,
        &["push", "origin", &format!("{moved}:refs/heads/auto/17")],
    )
    .unwrap();
    unchanged(&mut probe, "outside recorded publication");
}

#[test]
fn payload() {
    if !isolated("payload") {
        return;
    }
    let mut probe = failed();
    release("0.2.0");
    let file = probe.state.worktree.join("keep.txt");
    std::fs::write(&file, "valuable payload").unwrap();
    unchanged(&mut probe, "preserved local payload");
    assert_eq!(std::fs::read_to_string(file).unwrap(), "valuable payload");
}
