use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn seat(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-lock-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("skills").join("plumb")).expect("seat");
    fs::write(
        path.join("Cargo.toml"),
        "[workspace.package]\nversion = \"1.2.3\"\n",
    )
    .expect("cargo");
    fs::write(path.join("skills/plumb/SKILL.md"), "brief\n").expect("brief");
    path
}

fn declare(root: &Path, version: &str, hash: &str) {
    let text = format!(
        "[[lock]]\nname = \"skill\"\npaths = [\"skills/plumb\"]\nversion = \"{version}\"\nhash = \"{hash}\"\n"
    );
    fs::write(root.join("plumb.toml"), text).expect("declare");
}

fn plumb(root: &Path, verb: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([verb, root.to_str().expect("path")])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn hash(root: &Path) -> String {
    let shown = plumb(root, "lock");
    shown
        .lines()
        .find(|line| line.contains("hash"))
        .and_then(|line| line.split('"').nth(1))
        .expect("hash")
        .to_string()
}

#[test]
fn affirmed() {
    let root = seat("affirmed");
    declare(&root, "1.2.3", "seed");
    let held = hash(&root);
    declare(&root, "1.2.3", &held);
    assert!(
        !plumb(&root, "doctor").contains("lock skill"),
        "a fresh affirmation is quiet"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn drifted() {
    let root = seat("drifted");
    declare(&root, "1.2.3", "seed");
    let held = hash(&root);
    declare(&root, "1.2.3", &held);
    fs::write(root.join("skills/plumb/SKILL.md"), "brief again\n").expect("edit");
    let shown = plumb(&root, "doctor");
    assert!(
        shown.contains("have changed since the affirmation"),
        "{shown}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn moved() {
    let root = seat("moved");
    declare(&root, "1.2.3", "seed");
    let held = hash(&root);
    declare(&root, "1.0.0", &held);
    let shown = plumb(&root, "doctor");
    assert!(shown.contains("the repository stands at 1.2.3"), "{shown}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn absent() {
    let root = seat("absent");
    declare(&root, "1.2.3", "seed");
    fs::remove_dir_all(root.join("skills")).expect("sweep");
    let shown = plumb(&root, "doctor");
    assert!(shown.contains("does not exist"), "{shown}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn crlf() {
    let root = seat("crlf");
    declare(&root, "1.2.3", "seed");
    let held = hash(&root);
    fs::write(root.join("skills/plumb/SKILL.md"), "brief\r\n").expect("windows");
    assert_eq!(
        hash(&root),
        held,
        "a checkout that carries crlf digests the same"
    );
    let _ = fs::remove_dir_all(&root);
}
