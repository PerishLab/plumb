use super::settlement::seed;
use super::world::{Court, serve};
use std::process::{Command, Output};

fn run(fixture: &tempfile::TempDir, forge: &str, settled: &std::path::Path) -> Output {
    let path = format!(
        "{}:{}",
        fixture.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["version", "rejoin", "--version", "v1.2.0"])
        .current_dir(fixture.path())
        .env("PATH", path)
        .env("FORGEJO_TOKEN", "test-token")
        .env("FORGEJO_URL", forge)
        .env("COURT_ROOT", fixture.path())
        .env("COURT_CALLS", fixture.path().join("calls"))
        .env("COURT_SETTLED", settled)
        .env("COURT_STALE", fixture.path().join("stale"))
        .env("COURT_DIVERGED", fixture.path().join("diverged"))
        .output()
        .expect("plumb")
}

fn fixture() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("fixture");
    std::fs::write(fixture.path().join("resume"), "open pull\n").expect("resume marker");
    std::fs::write(fixture.path().join("stale"), "main advanced\n").expect("stale marker");
    seed(fixture.path());
    fixture
}

#[test]
fn retargets() {
    let fixture = fixture();
    let settled = fixture.path().join("settled");
    let (forge, held) = serve(Court::Rejoin(settled.clone()), 10);
    let output = run(&fixture, &forge, &settled);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(fixture.path().join("calls")).expect("calls");
    assert!(
        calls.contains(
            "git merge-base --is-ancestor aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa origin/main"
        ),
        "{calls}"
    );
    assert!(
        calls.contains(
            "git commit-tree eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee -p cccccccccccccccccccccccccccccccccccccccc -p bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        ),
        "{calls}"
    );
    assert!(
        calls.contains("git push --force-with-lease origin"),
        "{calls}"
    );
    let held = held.lock().expect("forge calls");
    assert!(
        !held
            .iter()
            .any(|call| call.starts_with("POST ") && call.contains("/pulls HTTP/1.1")),
        "{held:?}"
    );
    assert!(held.iter().any(|call| call.contains("/pulls/12/merge")));
}

#[test]
fn divergence() {
    let fixture = fixture();
    let settled = fixture.path().join("settled");
    std::fs::write(fixture.path().join("diverged"), "main diverged\n").expect("diverged marker");
    let (forge, _) = serve(Court::Rejoin(settled.clone()), 8);
    let output = run(&fixture, &forge, &settled);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("prove the projected main remains in current main failed"),
        "{error}"
    );
    let calls = std::fs::read_to_string(fixture.path().join("calls")).expect("calls");
    assert!(!calls.contains("git commit-tree"), "{calls}");
    assert!(!calls.contains("git push"), "{calls}");
}
