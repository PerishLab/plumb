use std::process::Command;

#[path = "doctor/dependency.rs"]
mod dependency;
#[path = "doctor/document.rs"]
mod document;
#[path = "doctor/json.rs"]
mod json;
#[path = "doctor/migration.rs"]
mod migration;
#[path = "doctor/rule.rs"]
mod rule;
#[path = "doctor/ships.rs"]
mod ships;

fn seat() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate should sit two below the repo")
        .to_path_buf()
}

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

fn fixture() -> tempfile::TempDir {
    let seat = tempfile::tempdir().expect("fixture");
    govern(seat.path());
    seat
}

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn version() {
    assert!(run(&["--version"]).contains("plumb v"));
}

#[test]
fn adaptors() {
    let listed = run(&["ship", "--help"]);
    for adaptor in ["binary", "cargo", "chart", "npm", "oci", "site"] {
        assert!(listed.contains(adaptor), "{listed}");
    }
    for adaptor in ["cargo", "chart", "npm", "oci", "site"] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["ship", adaptor])
            .output()
            .expect("plumb should run");
        assert!(!output.status.success());
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(
            text.contains(&format!(
                "the {adaptor} adaptor is declared and not absorbed"
            )),
            "{text}"
        );
    }
}

#[test]
#[ignore = "exercises live first-party registries in the repository guard"]
fn itself() {
    let out = run(&["doctor", seat().to_str().expect("path should be utf8")]);
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn governs() {
    let bare = std::env::temp_dir().join("plumb-ungoverned");
    std::fs::create_dir_all(&bare).expect("fixture should be made");
    govern(&bare);
    std::fs::write(bare.join("README.md"), "# bare\n").expect("readme should be written");
    let out = run(&["doctor", bare.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&bare).expect("fixture should be swept");
    assert!(!out.contains("no guard workflow"), "{out}");
    assert!(!out.contains("no ectropy.toml"), "{out}");
    assert!(out.contains("true to the skeleton"), "{out}");

    let seat = std::env::temp_dir().join("plumb-governed");
    std::fs::create_dir_all(seat.join(".runseal")).expect("fixture should be made");
    govern(&seat);
    let out = run(&["doctor", seat.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&seat).expect("fixture should be swept");
    assert!(out.contains("no guard workflow"), "{out}");
    assert!(out.contains("no ectropy.toml"), "{out}");
}

#[test]
fn concurrency() {
    let dir = std::env::temp_dir().join("plumb-concurrency");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join("runseal.toml"), "").expect("profile should be written");
    std::fs::write(dir.join("ectropy.toml"), "").expect("laws should be written");
    let lane = dir.join(".forgejo/workflows/guard.yml");

    std::fs::write(
        &lane,
        "name: guard\non:\n  pull_request:\n\njobs:\n  guard: {}\n",
    )
    .expect("lane should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        bare.contains(".forgejo/workflows/guard.yml lacks the concurrency block"),
        "{bare}"
    );

    let complete = "name: guard\non:\n  pull_request:\n\nconcurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true\n\njobs:\n  guard: {}\n";
    std::fs::write(&lane, complete).expect("lane should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::write(&lane, complete.replace('\n', "\r\n")).expect("lane should be written");
    let windows = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("concurrency block"), "{held}");
    assert!(!windows.contains("concurrency block"), "{windows}");
}

#[test]
fn authorities() {
    let fixture = fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join(".runseal")).expect("runseal should be made");
    std::fs::create_dir_all(root.join(".github/workflows")).expect("github should be made");
    std::fs::create_dir_all(root.join(".forgejo/workflows")).expect("forgejo should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("laws should be written");
    let complete = "name: quality\nconcurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true\njobs:\n  quality:\n    steps:\n      - run: plumb doctor . && ectropy .\n";
    let github = root.join(".github/workflows/quality.yml");
    let forgejo = root.join(".forgejo/workflows/guard.yml");

    std::fs::write(&github, complete).expect("github guard should be written");
    let hosted = run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(!hosted.contains("no guard workflow"), "{hosted}");
    assert!(!hosted.contains("concurrency block"), "{hosted}");

    std::fs::write(&forgejo, complete).expect("forgejo guard should be written");
    let ambiguous = run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(
        ambiguous.contains("multiple guard workflows"),
        "{ambiguous}"
    );

    std::fs::remove_file(&github).expect("github guard should be removed");
    let forgejo = run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(!forgejo.contains("no guard workflow"), "{forgejo}");
    assert!(!forgejo.contains("multiple guard workflows"), "{forgejo}");
}

#[test]
fn edition() {
    let dir = std::env::temp_dir().join("plumb-edition");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    govern(&dir);
    let cargo = dir.join("Cargo.toml");

    std::fs::write(&cargo, "[workspace.package]\nedition = \"2021\"\n")
        .expect("manifest should be written");
    let old = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        old.contains("edition is 2021, the skeleton holds 2024"),
        "{old}"
    );

    std::fs::write(&cargo, "[workspace.package]\nedition = \"2024\"\n")
        .expect("manifest should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("edition is"), "{held}");
}

#[test]
fn container() {
    let dir = std::env::temp_dir().join("plumb-container");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    let lane = dir.join(".forgejo/workflows/guard.yml");

    std::fs::write(&lane, "container: mirror.perish.lan/ci/deno:20260716-abc\n")
        .expect("lane should be written");
    let pin = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(pin.contains("CI container pinned to a tag"), "{pin}");

    std::fs::write(&lane, "container: mirror.perish.lan/ci/deno\n")
        .expect("lane should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!bare.contains("CI container pinned"), "{bare}");
}

#[test]
fn clap() {
    let dir = std::env::temp_dir().join("plumb-clap");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    let cargo = dir.join("Cargo.toml");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\ntoml = \"0.9\"\n",
    )
    .expect("manifest should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(bare.contains("ships a rust binary without clap"), "{bare}");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\nclap = \"4\"\n",
    )
    .expect("manifest should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);

    std::fs::write(&cargo, "[dependencies]\ntoml = \"0.9\"\n").expect("manifest should be written");
    let lib = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("without clap"), "{held}");
    assert!(!lib.contains("without clap"), "{lib}");
}

#[test]
fn substrate() {
    let dir = std::env::temp_dir().join("plumb-substrate");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    govern(&dir);
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    let cargo = dir.join("Cargo.toml");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\nclap = \"4\"\n",
    )
    .expect("manifest should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(bare.contains("ships a rust binary without plumb"), "{bare}");

    std::fs::write(
        dir.join("Cargo.lock"),
        "[[package]]\nname = \"plumb\"\nversion = \"0.2.0\"\n",
    )
    .expect("lock should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("without plumb"), "{held}");
}

#[test]
fn hookless() {
    let dir = std::env::temp_dir().join("plumb-hookless");
    std::fs::create_dir_all(dir.join(".runseal")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    govern(&dir);
    std::fs::write(
        dir.join(".forgejo/workflows/guard.yml"),
        "name: guard\nconcurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true\njobs:\n  guard:\n    steps:\n      - run: plumb doctor . && ectropy .\n",
    )
    .expect("workflow should be written");
    let policy = run(&["policy", dir.to_str().expect("path should be utf8")]);
    std::fs::write(dir.join("ectropy.toml"), policy).expect("policy should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(held.contains("true to the skeleton"), "{held}");
}
