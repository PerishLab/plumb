use super::world::{Fixture, run};
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
    fs::read_to_string(fixture.root.join(".forgejo/workflows/ship.yml")).expect("ship")
}

#[test]
fn derived() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    let ship = rendered(&fixture);

    assert!(ship.contains("on:\n  workflow_dispatch:"), "{ship}");
    assert!(ship.contains("marker:"), "{ship}");
    assert!(ship.contains("repository:"), "{ship}");
    assert!(!ship.contains("guard_contexts"), "{ship}");
    assert!(!ship.contains("release evidence"), "{ship}");
    assert!(!ship.contains("{@"), "{ship}");
}

#[test]
fn expressions() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    let ship = rendered(&fixture);
    assert!(
        ship.contains("-ship-${{ inputs.marker || github.ref }}"),
        "{ship}"
    );
    assert!(!ship.contains("cargo fmt"), "{ship}");
}

#[test]
fn drifts() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    rendered(&fixture);
    let path = temp.path().join(".forgejo/workflows/ship.yml");
    let held = fs::read_to_string(&path).expect("ship");
    fs::write(&path, held.replacen("name: ship", "name: drifted", 1)).expect("hand edit");

    let output = fixture
        .command()
        .args(["lane"])
        .arg(temp.path())
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("drifted .forgejo/workflows/ship.yml"),
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
        String::from_utf8_lossy(&output.stdout).contains("true .forgejo/workflows/ship.yml"),
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
        held.contains("${{ secrets.RELEASE_PUBLISH_S3_ACCESS_KEY }}"),
        "a callee still reads what its caller passes: {held}"
    );
    assert!(
        held.matches("/manage.sh | sh").count() == held.matches(">> \"$GITHUB_PATH\"").count(),
        "every install seats the home bin on the job path: {held}"
    );
    assert!(held.contains("\n  build:\n"), "{held}");
    assert!(held.contains("\n  seal:\n"), "{held}");
    assert!(held.contains("\n  smoke:\n"), "{held}");
    assert!(held.contains("plumb workflow record"), "{held}");
    assert!(held.contains("binary_reuse"), "{held}");
    assert!(held.contains("WORKFLOW_INVENTORY_URL"), "{held}");
    assert!(held.contains("packages/*/package.json"), "{held}");
    assert!(held.contains(".version | type == \"string\""), "{held}");
    assert!(
        held.contains("projection=Cargo.toml#/workspace/package/version"),
        "{held}"
    );
    assert!(held.contains("needs: [resolve, seal]"), "{held}");
    assert!(
        held.match_indices("if:").all(|(at, _)| {
            held[at..].starts_with("if: runner.os")
                || held[at..].starts_with("if: needs.resolve.outputs.channel")
                || held[at..].starts_with("if: >-\n      always() && needs.resolve.result")
        }),
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
        held.contains("PLUMB_SITE_TOKEN: ${{ secrets.PLUMB_SITE_TOKEN }}"),
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
        absent.contains("without its legacy workflow") && absent.contains("exact.release.yml"),
        "the lane a dispatch must reach cannot be missing: {absent}"
    );

    rendered(&fixture);
    let path = temp.path().join(".forgejo/workflows/ship.yml");
    let held = fs::read_to_string(&path).expect("ship");
    fs::write(
        &path,
        held.replace("Install bootstrap Plumb", "Install Plumb"),
    )
    .expect("hand edit");
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

    for name in ["exact.release.yml", "stable.release.yml", "depot.yml"] {
        assert!(
            !temp.path().join(".forgejo/workflows").join(name).exists(),
            "{name} is no longer an independent workflow"
        );
    }
    let held = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    assert!(held.contains("workflow_dispatch:"), "{held}");
    assert!(held.contains("marker:"), "{held}");
    assert!(!held.contains("\n  push:\n"), "{held}");
}
