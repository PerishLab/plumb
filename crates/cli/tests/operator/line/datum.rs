use super::stable::{command, repo, run};
use super::world::{Court, serve};
use std::path::Path;
use std::process::Command;

pub fn lined(root: &Path, origin: &str, bare: &Path, line: &str) -> String {
    run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
    let local = format!("file://{}", bare.display());
    repo(root, &local);
    let forge = origin
        .strip_suffix("/test/probe.git")
        .expect("fixture origin");
    run(Command::new("git")
        .args(["config", "plumb.test-forgejo-url", forge])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.email", "probe@test"])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.name", "probe"])
        .current_dir(root));
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Stand the probe up"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", &format!("HEAD:refs/heads/{line}")])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/main"])
        .current_dir(root));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("git");
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    head
}

#[test]
fn protection() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let cut = fixture.path().join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 7);
    let origin = format!("{url}/test/probe.git");
    let head = lined(fixture.path(), &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let output = command(
        fixture.path(),
        &["release", "prepare", "--version", "1.2.0"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stood = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stood.contains("prepared release/v1.2.0 from main"),
        "{stood}"
    );
    assert!(
        stood.contains(".plumb/releases/v1.2.0/datum.toml"),
        "{stood}"
    );
    let shown = Command::new("git")
        .args(["show", "release/v1.2.0:.plumb/releases/v1.2.0/datum.toml"])
        .current_dir(bare.path())
        .output()
        .expect("git");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let datum = String::from_utf8_lossy(&shown.stdout).to_string();
    assert!(datum.contains("schema = 1"), "{datum}");
    assert!(datum.contains("version = \"v1.2.0\""), "{datum}");
}

#[test]
fn sweeps() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 9);
    let origin = format!("{url}/test/probe.git");
    std::fs::create_dir_all(root.join(".plumb/releases/v1.2.0")).expect("seat");
    std::fs::write(
        root.join(".plumb/releases/v1.2.0/datum.json"),
        "{\"schema\":1}\n",
    )
    .expect("stale datum");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");

    let output = command(root, &["release", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("recorded"),
        "a seat holding a stray leaf is not already recorded"
    );
    let listed = Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            "release/v1.2.0",
            "--",
            ".plumb/releases/v1.2.0",
        ])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let listed = String::from_utf8_lossy(&listed.stdout).to_string();
    assert!(listed.contains("datum.toml"), "{listed}");
    assert!(
        !listed.contains("datum.json"),
        "the datum commit owns its seat: {listed}"
    );
}

#[test]
fn versions() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 14);
    let origin = format!("{url}/test/probe.git");
    lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(root.join("plumb.toml"), RELEASE).expect("release");
    for path in [
        "crates/macro",
        "crates/lib",
        "packages/probe",
        "charts/probe",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("seat");
    }
    std::fs::write(root.join("Cargo.toml"), CARGO).expect("workspace");
    std::fs::write(root.join("crates/macro/Cargo.toml"), MACRO).expect("macro");
    std::fs::write(root.join("crates/macro/lib.rs"), "").expect("macro source");
    std::fs::write(root.join("crates/lib/Cargo.toml"), LIB).expect("lib");
    std::fs::write(root.join("crates/lib/lib.rs"), "").expect("lib source");
    std::fs::write(root.join("packages/probe/package.json"), PACKAGE).expect("package");
    std::fs::write(root.join("charts/probe/Chart.yaml"), CHART).expect("chart");
    run(Command::new("cargo")
        .arg("generate-lockfile")
        .current_dir(root));
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Stand version seats up"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/main"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/release/v1.2.0"])
        .current_dir(root));
    let head = show(root, "--format=%H --no-patch HEAD").trim().to_string();
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    std::fs::write(&cut, &head).expect("cut");

    let output = command(root, &["release", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cargo = show(bare.path(), "release/v1.2.0:Cargo.toml");
    let library = show(bare.path(), "release/v1.2.0:crates/lib/Cargo.toml");
    let lock = show(bare.path(), "release/v1.2.0:Cargo.lock");
    let package = show(bare.path(), "release/v1.2.0:packages/probe/package.json");
    let chart = show(bare.path(), "release/v1.2.0:charts/probe/Chart.yaml");
    assert!(cargo.contains("version = \"1.2.0\""), "{cargo}");
    assert!(library.contains("version = \"=1.2.0\""), "{library}");
    assert_eq!(lock.matches("version = \"1.2.0\"").count(), 2, "{lock}");
    assert_eq!(package, PACKAGE.replace("1.1.0", "1.2.0"));
    assert!(chart.contains("version: 1.2.0"), "{chart}");
    assert!(chart.contains("appVersion: \"1.2.0\""), "{chart}");
    let touched = show(bare.path(), "--name-only --format= release/v1.2.0");
    assert!(
        touched
            .lines()
            .all(|path| path.starts_with(".plumb/releases/")),
        "datum must remain its own commit: {touched}"
    );
    let first = show(bare.path(), "--format=%H --no-patch release/v1.2.0")
        .trim()
        .to_string();
    std::fs::write(&cut, &first).expect("current cut");
    let repeated = command(root, &["release", "prepare", "--version", "1.2.0"]);
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    let second = show(bare.path(), "--format=%H --no-patch release/v1.2.0");
    assert_eq!(
        first,
        second.trim(),
        "repeated prepare must not move the line"
    );
}

fn show(root: &Path, object: &str) -> String {
    let mut args = vec!["show"];
    args.extend(object.split_whitespace());
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git show");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

const RELEASE: &str = r#"[release]
product = "probe"
authority = "https://releases.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]

[release.cargo]
registry = "perish"
packages = ["probe-macro", "probe-lib"]

[release.npm]
registry = "https://registry.test/npm/"
packages = ["@test/probe"]

[release.chart]
registry = "registry.test"
chart = "test/probe"
account = "test"
"#;

const CARGO: &str = r#"[workspace]
members = ["crates/macro", "crates/lib"]
resolver = "3"

[workspace.package]
version = "1.1.0"
edition = "2024"
"#;

const MACRO: &str = r#"[package]
name = "probe-macro"
version.workspace = true
edition.workspace = true

[lib]
path = "lib.rs"
"#;

const LIB: &str = r#"[package]
name = "probe-lib"
version.workspace = true
edition.workspace = true

[lib]
path = "lib.rs"

[dependencies]
probe-macro = { path = "../macro", version = "=1.1.0" }
"#;

const PACKAGE: &str = "{\n\t\"name\": \"@test/probe\",\n\t\"version\": \"1.1.0\"\n}\n";
const CHART: &str = "name: probe\nversion: 1.1.0\nappVersion: \"1.1.0\"\n";
