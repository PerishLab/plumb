use std::path::Path;
use std::process::{Command, Output};

const PROFILE: &str = "2f1639e5915be5049de25201dfe2d7e2958be4ab1484b41bba96aee832152ef9";

#[test]
fn closed() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path().join("runseal");
    std::fs::create_dir_all(root.join("crates/cli/src")).expect("CLI source");
    std::fs::create_dir_all(root.join("crates/lib/src")).expect("library source");
    git(&root, &["init", "-q"]);
    git(
        &root,
        &[
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishFire/runseal.git",
        ],
    );
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    std::fs::write(root.join("AGENTS.md"), "# Runseal\n").expect("operations");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/cli\", \"crates/lib\"]\nresolver = \"3\"\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("workspace");
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"plumb\"\nversion = \"0.1.0\"\n",
    )
    .expect("lock");
    std::fs::write(
        root.join("crates/cli/Cargo.toml"),
        "[package]\nname = \"runseal-cli\"\nversion.workspace = true\nedition.workspace = true\npublish = false\n[[bin]]\nname = \"runseal\"\npath = \"src/main.rs\"\n[dependencies]\nclap = \"4\"\n",
    )
    .expect("CLI manifest");
    std::fs::write(root.join("crates/cli/src/main.rs"), "fn main() {}\n").expect("CLI");
    std::fs::write(
        root.join("crates/lib/Cargo.toml"),
        "[package]\nname = \"runseal\"\nversion.workspace = true\nedition.workspace = true\npublish = [\"perish\"]\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .expect("library manifest");
    std::fs::write(root.join("crates/lib/src/lib.rs"), "pub fn run() {}\n").expect("library");
    git(&root, &["add", "-A"]);
    super::super::world::hooks(&root);

    let depot = super::super::support::depot(&[]);
    let output = plumb(&root, depot.path(), &["doctor", ".", "--json"]);
    assert!(output.status.success(), "{}", text(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["profile"], PROFILE);
    assert_eq!(report["ok"], true);
    assert!(!root.join("plumb.toml").exists());
    assert!(!root.join("ectropy.toml").exists());

    let surface = plumb(&root, depot.path(), &["ship", "surface"]);
    assert!(surface.status.success(), "{}", text(&surface));
    let surface: serde_json::Value = serde_json::from_slice(&surface.stdout).expect("surface");
    let operations = surface["publication"]["include"]
        .as_array()
        .expect("publication rows");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0]["operation"]["type"], "cargo");
}

fn plumb(root: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_RELEASE_ROOT", root)
        .output()
        .expect("Plumb")
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("Git");
    assert!(output.status.success(), "{}", text(&output));
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
