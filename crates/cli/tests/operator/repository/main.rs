#[cfg(unix)]
mod audit;
mod chart;
#[cfg(unix)]
mod module;
mod precommit;
#[cfg(unix)]
#[path = "registry/main.rs"]
mod registry;
#[cfg(unix)]
mod site;
mod surface;
mod world;

#[path = "../../support.rs"]
mod support;

use world::{govern, run};

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
    assert!(!bare.contains("guard does not run plumb doctor"), "{bare}");
    assert!(
        !bare.contains("guard does not run ectropy explicitly"),
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
        !bare.contains("guard does not exercise the release profile"),
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
        !held.contains("guard uses an obsolete ectropy mode"),
        "{held}"
    );
}
