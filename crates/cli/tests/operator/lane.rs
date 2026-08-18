use super::fixture::{Fixture, run};
use std::fs;
use std::path::Path;
use std::process::Command;

pub const CARGO: &str = "[workspace]\nmembers = []\n";

pub fn seat<'a>(root: &'a Path, tools: &'a Path) -> Fixture<'a> {
    fs::create_dir_all(tools).expect("tool root");
    let held = Fixture { root, tools };
    held.seed();
    held
}

pub fn rendered(fixture: &Fixture<'_>) -> String {
    let output = fixture
        .command()
        .args(["lane", "--write"])
        .arg(fixture.root)
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(fixture.root.join(".forgejo/workflows/guard.yml")).expect("guard")
}

#[test]
fn derived() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    let guard = rendered(&fixture);

    assert!(guard.contains("on:\n  pull_request:\n"), "{guard}");
    assert!(guard.contains("forge@sha256:"), "{guard}");
    assert!(!guard.contains("mirror.perish.lan"), "{guard}");
    assert!(!guard.contains("setup-binary"), "{guard}");
    assert!(
        guard.contains("curl -fsSL https://releases.ectropy.perish.uk/manage.sh | sh"),
        "{guard}"
    );
    assert!(
        guard.contains("curl -fsSL https://releases.plumb.perish.uk/manage.sh | sh"),
        "{guard}"
    );
    assert!(
        guard.contains("printf '%s\\n' \"$HOME/.local/bin\" >> \"$GITHUB_PATH\""),
        "a stable tool installs under the home seat, so the job path must carry it: {guard}"
    );
    assert!(guard.contains("cargo fmt --all --check"), "{guard}");
    assert!(guard.contains("plumb doctor ."), "{guard}");
    assert!(guard.contains("ectropy ."), "{guard}");
    assert!(!guard.contains("pnpm"), "{guard}");
    assert!(!guard.contains("{@"), "{guard}");
}

#[test]
fn expressions() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    let guard = rendered(&fixture);
    assert!(
        guard.contains("group: guard-${{ github.event.pull_request.number || github.ref }}"),
        "{guard}"
    );
    assert!(!guard.contains("cargo fmt"), "{guard}");
}

#[test]
fn drifts() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    rendered(&fixture);
    let path = temp.path().join(".forgejo/workflows/guard.yml");
    let held = fs::read_to_string(&path).expect("guard");
    fs::write(&path, held.replace("Checkout", "Checkout the source")).expect("hand edit");

    let output = fixture
        .command()
        .args(["lane"])
        .arg(temp.path())
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("drifted .forgejo/workflows/guard.yml"),
        "{text}"
    );

    rendered(&fixture);
    let output = fixture
        .command()
        .args(["lane"])
        .arg(temp.path())
        .output()
        .expect("plumb should run");
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("true .forgejo/workflows/guard.yml"),
        "rewriting a drifted lane must settle it"
    );
}

const ATTACHED: &str = "[release.cargo]\nregistry = \"perish\"\npackages = [\"probe\"]\n";

fn ship(root: &Path) -> String {
    fs::read_to_string(root.join(".forgejo/workflows/ship.yml")).expect("ship")
}

#[test]
fn carried() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    rendered(&fixture);
    let held = ship(temp.path());

    assert!(
        !held.contains("\n    secrets:\n"),
        "this forge parses workflow_call inputs and outputs only, so a callee declares no secrets: {held}"
    );
    assert!(
        held.contains("${{ secrets.publish_access }}"),
        "a callee still reads what its caller passes: {held}"
    );
    assert!(
        held.matches("/manage.sh | sh").count() == held.matches(">> \"$GITHUB_PATH\"").count(),
        "every install seats the home bin on the job path: {held}"
    );
    assert!(held.contains("\n  build:\n"), "{held}");
    assert!(held.contains("\n  seal:\n"), "{held}");
    assert!(held.contains("\n  smoke:\n"), "{held}");
    assert!(held.contains("needs: [resolve, seal]"), "{held}");
    assert!(
        held.match_indices("if:")
            .all(|(at, _)| held[at..].starts_with("if: runner.os")),
        "a rendered lane carries no condition render time could have decided: {held}"
    );
    assert!(!held.contains("{@"), "{held}");
}

#[test]
fn attached() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("plumb.toml"), ATTACHED).expect("manifest");
    rendered(&fixture);
    let held = ship(temp.path());

    assert!(!held.contains("\n  build:\n"), "{held}");
    assert!(!held.contains("\n  seal:\n"), "{held}");
    assert!(!held.contains("\n  smoke:\n"), "{held}");
    assert!(held.contains("\n  project:\n"), "{held}");
    assert!(held.contains("needs: [resolve]"), "{held}");
    assert!(
        held.contains("PLUMB_SITE_TOKEN: ${{ secrets.site_token }}"),
        "a projected medium may be a worker: {held}"
    );
    assert!(
        !held.contains("include\":[]"),
        "no empty matrix is rendered"
    );
}

#[test]
fn refused() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("plumb.toml"), ATTACHED).expect("manifest");
    run(Command::new("git").arg("-C").arg(temp.path()).args([
        "remote",
        "add",
        "origin",
        "ssh://git@git.perish.top/PerishLab/probe.git",
    ]));

    let bare = dispatch(&fixture);
    let absent = String::from_utf8_lossy(&bare.stderr);
    assert!(!bare.status.success());
    assert!(
        absent.contains("has not rendered") && absent.contains("exact.release.yml"),
        "the lane a dispatch must reach cannot be missing: {absent}"
    );

    rendered(&fixture);
    let path = temp.path().join(".forgejo/workflows/ship.yml");
    let held = fs::read_to_string(&path).expect("ship");
    fs::write(&path, held.replace("Install stable Plumb", "Install Plumb")).expect("hand edit");
    let stale = dispatch(&fixture);
    assert!(!stale.status.success());
    let text = String::from_utf8_lossy(&stale.stderr);
    assert!(text.contains("did not render"), "{text}");
    assert!(text.contains("ship.yml"), "{text}");
}

fn dispatch(fixture: &Fixture<'_>) -> std::process::Output {
    fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ])
        .output()
        .expect("plumb should run")
}

#[test]
fn anchored() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    rendered(&fixture);

    for name in ["exact.release.yml", "stable.release.yml"] {
        let held = fs::read_to_string(temp.path().join(".forgejo/workflows").join(name))
            .expect("release lane");
        assert!(
            held.contains("on:\n  workflow_dispatch:\n"),
            "{name} must wait for an operator: {held}"
        );
        assert!(
            !held.contains("\n  push:\n"),
            "a release never follows from a push: {name}: {held}"
        );
        assert!(
            held.contains("guard_contexts: '[\"guard / guard (push)\"]'"),
            "{name} must forward the canonical guard evidence: {held}"
        );
        assert!(
            held.contains("site_token: ${{ secrets.PLUMB_SITE_TOKEN }}"),
            "{name} must forward the credential a worker medium needs: {held}"
        );
    }
}
