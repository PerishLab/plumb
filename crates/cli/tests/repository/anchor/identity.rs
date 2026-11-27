use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", crate::support::home().keep())
        .args(args)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

fn workspace(root: &Path, member: &str, name: &str) {
    let seat = root.join("crates").join(member);
    std::fs::create_dir_all(seat.join("src")).expect("crate should be made");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nmembers = [\"crates/{member}\"]\n"),
    )
    .expect("workspace should be written");
    std::fs::write(
        seat.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\n"),
    )
    .expect("manifest should be written");
}

#[test]
fn identified() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("w3-plectron-repo");
    std::fs::create_dir_all(dir.join("crates/lib/src")).expect("lib should be made");
    std::fs::create_dir_all(dir.join("crates/other/src")).expect("other should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(dir.join("plumb.toml"), "[identity]\nns = \"plectron\"\n")
        .expect("plumb manifest should be written");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/lib\", \"crates/other\"]\n",
    )
    .expect("workspace should be written");
    std::fs::write(
        dir.join("crates/lib/Cargo.toml"),
        "[package]\nname = \"plectron\"\n",
    )
    .expect("lib manifest should be written");
    std::fs::write(
        dir.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n",
    )
    .expect("other manifest should be written");
    std::fs::write(
        dir.join("crates/lib/src/lib.rs"),
        "#[derive(radicula_config::Cascade)]\nstruct Config;\n",
    )
    .expect("source should be written");

    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!held.contains("anchor is missing"), "{held}");
    assert!(!held.contains("anchor must be alone"), "{held}");
    assert!(!held.contains("outside the anchor"), "{held}");

    std::fs::write(
        dir.join("crates/other/src/lib.rs"),
        "#[derive(radicula_config::Cascade)]\nstruct Extra;\n",
    )
    .expect("source should be written");
    let strayed = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        strayed.contains("Cascade derives in crates/other, outside the anchor"),
        "{strayed}"
    );
}

#[test]
fn precedence() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("w3-precedence-repo");
    workspace(&dir, "anchor", "from-identity");
    git(&dir, &["init", "-q"]);
    git(
        &dir,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/from-origin.git",
        ],
    );
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/anchor\"]\n\
         [workspace.package]\nrepository = \"https://github.com/PerishLab/from-cargo.git\"\n",
    )
    .expect("workspace should be written");
    std::fs::write(
        dir.join("plumb.toml"),
        "[identity]\nns = \"from-identity\"\n",
    )
    .expect("plumb manifest should be written");

    let path = dir.to_str().expect("path should be utf8");
    let first = run(&["doctor", path]);
    assert!(!first.contains("anchor is missing"), "{first}");

    std::fs::remove_file(dir.join("plumb.toml")).expect("plumb manifest should be removed");
    std::fs::write(
        dir.join("crates/anchor/Cargo.toml"),
        "[package]\nname = \"from-cargo\"\n",
    )
    .expect("manifest should be written");
    let second = run(&["doctor", path]);
    assert!(!second.contains("anchor is missing"), "{second}");

    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/anchor\"]\n",
    )
    .expect("workspace should be written");
    std::fs::write(
        dir.join("crates/anchor/Cargo.toml"),
        "[package]\nname = \"from-origin\"\n",
    )
    .expect("manifest should be written");
    let third = run(&["doctor", path]);
    assert!(!third.contains("anchor is missing"), "{third}");

    git(&dir, &["remote", "remove", "origin"]);
    let fallback = run(&["doctor", path]);
    assert!(
        fallback.contains("no crate is named w3-precedence-repo, the anchor is missing"),
        "{fallback}"
    );
}

#[test]
fn remote() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("w3-remote-repo");
    workspace(&dir, "lib", "plectron");
    std::fs::write(
        dir.join("crates/lib/src/lib.rs"),
        "#[derive(radicula_config::Cascade)]\nstruct Config;\n",
    )
    .expect("source should be written");
    git(&dir, &["init", "-q"]);
    git(
        &dir,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/plectron.git",
        ],
    );

    let path = dir.to_str().expect("path should be utf8");
    for url in [
        "https://github.com/PerishLab/plectron.git",
        "https://github.com/PerishLab/plectron/",
        "git@github.com:PerishLab/plectron.git",
        "ssh://git@github.com/PerishLab/plectron.git",
    ] {
        git(&dir, &["remote", "set-url", "origin", url]);
        let held = run(&["doctor", path]);
        assert!(!held.contains("anchor is missing"), "{url}: {held}");
        assert!(!held.contains("outside the anchor"), "{url}: {held}");
    }
}

#[test]
fn worktree() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let main = scratch.path().join("w3-main-clone");
    let seat = scratch.path().join("w3-worktree-seat");
    workspace(&main, "lib", "plectron");
    git(&main, &["init", "-q"]);
    git(&main, &["config", "user.name", "Test"]);
    git(&main, &["config", "user.email", "test@example.invalid"]);
    git(
        &main,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/plectron.git",
        ],
    );
    git(&main, &["add", "."]);
    git(&main, &["commit", "-q", "-m", "init"]);
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "topic",
            seat.to_str().expect("seat should be utf8"),
        ],
    );

    let path = seat.to_str().expect("seat should be utf8");
    let held = run(&["doctor", path]);
    assert!(!held.contains("anchor is missing"), "{held}");

    git(&main, &["remote", "remove", "origin"]);
    let fallback = run(&["doctor", path]);
    assert!(
        fallback.contains("no crate is named w3-main-clone, the anchor is missing"),
        "{fallback}"
    );
}

#[test]
fn blank() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("w3-blank-repo");
    workspace(&dir, "lib", "plectron");
    std::fs::write(dir.join("plumb.toml"), "[identity]\nns = \"   \"\n")
        .expect("plumb manifest should be written");
    git(&dir, &["init", "-q"]);
    git(
        &dir,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/plectron.git",
        ],
    );

    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!held.contains("anchor is missing"), "{held}");
}
