use super::lane::{CARGO, rendered, seat};
use super::world::SPEC;
use std::fs;

#[test]
fn workload() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::create_dir_all(temp.path().join("packages/probe")).expect("module root");
    fs::write(
        temp.path().join("plumb.toml"),
        "[release.npm]\nregistry = \"https://registry.example\"\npackages = [\"probe\"]\n",
    )
    .expect("manifest");
    let held = rendered(&fixture);

    assert!(held.contains("plumb ship npm exact"), "{held}");
    assert!(held.contains("--root \"$action=$root\""), "{held}");
    assert!(held.contains("kind\" != url"), "{held}");
    assert!(
        held.contains("plumb workflow record '${{ matrix.action }}'"),
        "{held}"
    );
    assert!(
        held.contains("matrix.medium != 'npm' || matrix.reuse.type == 'none'"),
        "a held module workload must not install or build the source: {held}"
    );
    assert!(
        held.contains("\"control\":\"reuse\""),
        "a fully published project plan remains a resolvable Forgejo carrier: {held}"
    );
}

#[test]
fn planned() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    assert_eq!(
        ship.matches("\"$tool\" depot sync").count(),
        ship.matches("Install bootstrap Plumb").count(),
        "every Unix install must establish its rule seat: {ship}"
    );
    assert_eq!(
        ship.matches("plumb.exe') depot sync").count(),
        ship.matches("Install stable Plumb on Windows").count(),
        "every Windows install must establish its rule seat: {ship}"
    );
    assert!(
        !ship.contains("Build the exact atom Plumb"),
        "a product release must use published Plumb rather than build a binary it does not carry: {ship}"
    );
    let plan = ship
        .split("- name: Plan this release")
        .nth(1)
        .expect("plan step");
    let probe = plan
        .find("plumb release plan --help")
        .expect("plan capability probe");
    let derived = plan
        .find("plan=$(plumb release plan)")
        .expect("plan derivation");
    let version = plan
        .find("export PLUMB_RELEASE_VERSION")
        .expect("version export");
    assert!(
        probe < derived && derived < version,
        "a capable generator must derive the plan before exporting it: {plan}"
    );
    for deed in [
        "PLUMB_RELEASE_VERSION=$(plumb release reference)",
        "PLUMB_RELEASE_CHANNEL=$(plumb release channel)",
        "plumb release source",
        "plumb release surface",
    ] {
        assert!(
            plan.contains(deed),
            "a lane held by the preceding generator is missing legacy deed {deed}: {plan}"
        );
    }
}

#[test]
fn selfhosted() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    fs::write(
        temp.path().join("plumb.toml"),
        SPEC.replace("probe", "plumb"),
    )
    .expect("release manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    assert_eq!(
        ship.matches("- name: Build the exact atom Plumb\n").count(),
        ship.matches("Install bootstrap Plumb").count(),
        "every self-hosting bootstrap must hand execution to the source Plumb: {ship}"
    );
    let bootstrap = ship.find("Install bootstrap Plumb").expect("bootstrap");
    let sync = ship.find("\"$tool\" depot sync").expect("depot sync");
    let source = ship
        .find("Build the exact atom Plumb")
        .expect("source build");
    let plan = ship.find("Plan this release").expect("release plan");
    assert!(bootstrap < sync && sync < source && source < plan, "{ship}");
    assert!(
        ship.contains("/${{ github.repository }}.git")
            && ship.contains("origin \"${{ github.sha }}\"")
            && ship.contains("--manifest-path \"$root/Cargo.toml\""),
        "the control atom must supply its own exact executable: {ship}"
    );
    assert!(
        !ship.contains("$GITHUB_WORKSPACE/target/debug/plumb"),
        "{ship}"
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
    assert!(
        ship.contains("Join-Path $env:RUNNER_TEMP 'plumb-bootstrap'")
            && !ship.contains("Join-Path $HOME '.local/bin/plumb.exe'"),
        "a reused Windows runner must isolate its bootstrap from the persistent home seat: {ship}"
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
fn promotion() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = seat(temp.path(), &tools);
    fs::write(temp.path().join("Cargo.toml"), CARGO).expect("cargo manifest");
    fs::write(
        temp.path().join("plumb.toml"),
        SPEC.replace("probe", "plumb"),
    )
    .expect("release manifest");
    rendered(&fixture);
    let ship = fs::read_to_string(temp.path().join(".forgejo/workflows/ship.yml")).expect("ship");
    let build = ship
        .split("\n  materialize:\n")
        .nth(1)
        .expect("materialize job");
    let build = build.split("\n  seal:\n").next().expect("materialize body");
    assert!(
        build.contains("if: needs.resolve.outputs.channel != 'stable'"),
        "stable promotion must not obtain a runner for binary build: {build}"
    );
    let smoke = ship.split("\n  verify:\n").nth(1).expect("verify job");
    let smoke = smoke.split("\n  project:\n").next().expect("verify body");
    assert!(
        smoke.contains("if: needs.resolve.outputs.channel != 'stable'"),
        "a beta-smoked binary must not obtain stable smoke runners: {smoke}"
    );
    let seal = ship.split("\n  seal:\n").nth(1).expect("seal job");
    let seal = seal.split("\n  smoke:\n").next().expect("seal body");
    let promote = seal
        .find("Fetch the derived promotion proof and its binary artifacts")
        .expect("promotion materialization");
    let assemble = seal
        .find("Assemble the declared binary shape")
        .expect("binary assembly");
    assert!(
        promote < assemble,
        "promotion artifacts must exist before assembly: {seal}"
    );
    assert!(
        seal.contains(
            "if: needs.resolve.outputs.channel != 'stable' && needs.resolve.outputs.binary_missing == 'true'\n        with:\n          path: dist/"
        ),
        "stable and fully reused seals must not wait on run-local binary artifacts: {seal}"
    );
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
    assert!(
        ship.contains("plumb release activate"),
        "shifting the managers is not advancing the pointer; a stable release does both: {ship}"
    );
}
