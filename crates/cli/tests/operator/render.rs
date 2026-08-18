use super::lane::{CARGO, rendered, seat};
use std::fs;
use std::path::Path;

#[test]
fn bootstrapped() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    rendered(&fixture);
    let ship = |root: &Path| {
        fs::read_to_string(root.join(".forgejo/workflows/ship.yml")).expect("ship lane")
    };
    let ordinary = ship(temp.path());
    assert!(
        !ordinary.contains("the Plumb this release is bound to"),
        "an ordinary product carries no bootstrap: {ordinary}"
    );

    fs::write(
        temp.path().join("plumb.toml"),
        "[release]\nproduct = \"plumb\"\nauthority = \"https://releases.plumb.perish.uk\"\nbinaries = [\"plumb\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
    )
    .expect("release manifest");
    rendered(&fixture);
    let held = ship(temp.path());
    assert!(
        held.contains("the Plumb this release is bound to"),
        "{held}"
    );
    assert!(
        held.contains("refs/heads/release/v0.26.0")
            && held.contains("--channel beta --version v0.26.0-beta.4"),
        "the bootstrap binds one line to one published beta: {held}"
    );
    assert!(!held.contains("{@"), "{held}");
}

#[test]
fn planned() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    let plan = ship
        .split("- name: Plan this release")
        .nth(1)
        .expect("plan step");
    let version = plan
        .find("export PLUMB_RELEASE_VERSION")
        .expect("version export");
    let channel = plan
        .find("plumb release channel")
        .expect("channel derivation");
    assert!(
        version < channel,
        "a derivation reading the version must run after the export that carries it: {plan}"
    );
}

#[test]
fn portable() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    for step in ship.split("      - name: ").skip(1) {
        let head = step.lines().next().unwrap_or_default().to_string();
        if step.contains("run: |") {
            assert!(
                step.contains("shell: bash") || step.contains("shell: pwsh"),
                "a step whose body is a script must name the shell it is written in: {head}"
            );
        }
    }
    assert!(
        ship.contains("rustup target add ${{ matrix.target }}"),
        "a matrix runner holds only its own target until one is added: {ship}"
    );
    assert_eq!(
        ship.matches("if: runner.os != 'Windows'").count(),
        ship.matches("if: runner.os == 'Windows'").count(),
        "every job a Windows runner reaches carries both halves of its install"
    );
    assert!(
        ship.contains("shell: pwsh") && ship.contains("manage.ps1"),
        "a Windows runner has no bash, so its install is written in its own shell: {ship}"
    );
}

#[test]
fn scoped() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    let seal = ship.split("\n  seal:\n").nth(1).expect("seal job");
    let job = seal.split("    steps:").next().expect("job header");
    assert!(
        !job.contains("PLUMB_RELEASE_PROMOTION"),
        "a proof only stable holds cannot sit in the env every step reads: {job}"
    );
    for step in seal.split("      - name: ").skip(1) {
        if step.contains("PLUMB_RELEASE_PROMOTION") {
            assert!(
                step.contains("if: needs.resolve.outputs.channel == 'stable'"),
                "the proof travels only with the steps that may hold it: {step}"
            );
        }
    }
}

#[test]
fn carried() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    fs::write(
        temp.path().join("plumb.toml"),
        "[release]\nproduct = \"probe\"\nauthority = \"https://releases.test\"\nbinaries = [\"probe\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n\n[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n",
    )
    .expect("release manifest");
    fs::create_dir_all(temp.path().join("charts/probe")).expect("chart seat");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    let project = ship.split("\n  project:\n").nth(1).expect("project job");
    assert!(
        project.contains("name: release-capsule"),
        "a projection that gates on the capsule must be handed one: {project}"
    );
    assert!(
        ship.contains("manager_unix") && ship.contains("manager_windows"),
        "a smoke runs against the manager its own platform was given: {ship}"
    );
    assert!(!ship.contains("binary manager\n"), "{ship}");
    assert!(
        project.contains("corepack enable"),
        "a projection that spends pnpm must first have one: {project}"
    );
}
