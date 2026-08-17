use std::process::Command;

#[cfg(unix)]
mod artifact;
mod audit;
#[cfg(unix)]
mod fixture;
#[cfg(unix)]
mod forgejo;
#[cfg(unix)]
mod lane;
#[cfg(unix)]
mod pin;
mod precommit;
#[cfg(unix)]
mod promotion;
#[cfg(unix)]
mod recovery;
#[cfg(unix)]
mod registry;
#[cfg(unix)]
mod release;
#[cfg(unix)]
mod retract;
#[cfg(unix)]
mod settlement;
#[cfg(unix)]
mod site;
#[cfg(unix)]
mod stable;
mod surface;
#[cfg(unix)]
mod topology;

fn govern(root: &std::path::Path) {
    let status = Command::new("git")
        .args([
            "-C",
            root.to_str().expect("path should be utf8"),
            "init",
            "-q",
        ])
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should become a repository");
}

fn run(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn actions() {
    let dir = std::env::temp_dir().join("plumb-actions");
    std::fs::create_dir_all(dir.join("setup-tool")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join("setup-tool/action.yml"), "name: setup\n")
        .expect("action should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        !held.contains("directory setup-tool has no shadow"),
        "{held}"
    );
}

#[test]
fn adoption() {
    let dir = std::env::temp_dir().join("plumb-adoption");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join("runseal.toml"), "").expect("profile should be written");
    let lane = dir.join(".forgejo/workflows/guard.yml");
    std::fs::write(
        &lane,
        "concurrency: guard-${{ github.event.pull_request.number || github.ref }}\n",
    )
    .expect("guard should be written");
    let bare = run(&dir);
    assert!(bare.contains("guard does not run plumb doctor"), "{bare}");
    assert!(
        bare.contains("guard does not run ectropy explicitly"),
        "{bare}"
    );

    std::fs::write(
        &lane,
        "concurrency: guard-${{ github.event.pull_request.number || github.ref }}\nrun: plumb doctor . && ectropy .\n",
    )
    .expect("guard should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("guard does not run plumb doctor"), "{held}");
    assert!(
        !held.contains("guard does not run ectropy explicitly"),
        "{held}"
    );
}

#[test]
fn cargo() {
    let dir = std::env::temp_dir().join("plumb-cargo-lane");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(
        dir.join(".forgejo/workflows/release-cargo.yml"),
        "name: release-cargo\n",
    )
    .expect("lane should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        !held.contains("workflow release-cargo has no shadow in the skeleton"),
        "{held}"
    );
}

#[test]
fn runner() {
    let dir = std::env::temp_dir().join("plumb-runner");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join("runner-control")).expect("runner seat should be made");
    govern(&dir);
    std::fs::write(
        dir.join(".forgejo/workflows/release-runner.yml"),
        "name: release-runner\n",
    )
    .expect("lane should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        !held.contains("directory runner-control has no shadow in the skeleton"),
        "{held}"
    );
    assert!(
        !held.contains("workflow release-runner has no shadow in the skeleton"),
        "{held}"
    );
}

#[test]
fn wrappers() {
    let fixture = tempfile::tempdir().expect("fixture should be made");
    govern(fixture.path());
    std::fs::create_dir_all(fixture.path().join(".runseal/wrappers"))
        .expect("wrapper seat should be made");
    std::fs::write(fixture.path().join(".runseal/wrappers/special.ts"), "")
        .expect("wrapper should be written");
    let held = run(fixture.path());
    assert!(held.contains("wrappers  special"), "{held}");
    assert!(!held.contains("wrapper special has no shadow"), "{held}");
}

#[test]
fn profile() {
    let dir = std::env::temp_dir().join("plumb-profile");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join("runseal.toml"), "").expect("profile should be written");
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\n").expect("manifest should be written");
    let lane = dir.join(".forgejo/workflows/guard.yml");
    std::fs::write(
        &lane,
        "concurrency: guard-${{ github.event.pull_request.number || github.ref }}\n",
    )
    .expect("guard should be written");
    let bare = run(&dir);
    assert!(
        bare.contains("guard does not exercise the release profile"),
        "{bare}"
    );

    std::fs::write(
        &lane,
        "concurrency: guard-${{ github.event.pull_request.number || github.ref }}\nrun: cargo check --workspace --release\n",
    )
    .expect("guard should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        !held.contains("guard does not exercise the release profile"),
        "{held}"
    );
}

#[test]
fn obsolete() {
    let dir = std::env::temp_dir().join("plumb-obsolete");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join("runseal.toml"), "").expect("profile should be written");
    std::fs::write(
        dir.join(".forgejo/workflows/guard.yml"),
        "concurrency: guard-${{ github.event.pull_request.number || github.ref }}\nrun: plumb doctor . && ectropy --strict .\n",
    )
    .expect("guard should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        held.contains("guard uses an obsolete ectropy mode"),
        "{held}"
    );
}
