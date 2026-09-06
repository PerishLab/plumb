const BINARY: &str = "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";

#[path = "../../../src/shape/pair/identity.rs"]
mod identity;

use std::process::Command;

fn seat(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    crate::govern(&dir);
    dir
}

#[test]
fn refused() {
    let dir = seat("plumb-release-declaration");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let whole = crate::run(&["doctor", path]);
    assert!(!whole.contains("the current Plumb refuses"), "{whole}");
    assert!(whole.contains("publishes binary"), "{whole}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = [\"@probe/foo\"]\n\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
    )
    .expect("manifest should be written");
    let captured = crate::run(&["doctor", "--json", path]);
    let report: serde_json::Value = serde_json::from_str(&captured).expect("doctor json");
    assert_eq!(report["summary"]["out_of_true"], 1, "{captured}");
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| {
                finding["code"] == "release.spec-declared"
                    && finding["evidence"].as_str().is_some_and(|evidence| {
                        evidence.contains("the current Plumb refuses")
                            && evidence.contains("cannot parse")
                    })
            })),
        "{captured}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn deliverable() {
    let dir = seat("plumb-release-attachment");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let uncalled = crate::run(&["doctor", path]);
    assert!(uncalled.contains("publishes binary"), "{uncalled}");
    assert!(!uncalled.contains("attachment is declared"), "{uncalled}");
    assert!(!dir.join(".forgejo").exists());

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.oci]\nregistry = \"example.invalid\"\nimage = \"perishlab/foo\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest should be written");
    let held = crate::run(&["doctor", path]);
    assert!(held.contains("publishes binary oci"), "{held}");
    assert!(!held.contains("attachment is declared"), "{held}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn carried() {
    let dir = seat("plumb-release-cargo-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let held = crate::run(&["doctor", path]);
    assert!(held.contains("publishes cargo"), "{held}");
    assert!(!held.contains("the current Plumb refuses"), "{held}");
    assert!(!held.contains("cargo attachment is declared"), "{held}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn depot() {
    let dir = seat("plumb-release-depot");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"configuration\", \"changelog\"]\nvalidator = [\"foo\", \"guard\"]\n"
        ),
    )
    .expect("manifest should be written");
    let held = crate::run(&["doctor", path]);
    assert!(!held.contains("the current Plumb refuses"), "{held}");

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"artifact\"]\n"
        ),
    )
    .expect("manifest should be written");
    let unknown = crate::run(&["doctor", path]);
    assert!(unknown.contains("the current Plumb refuses"), "{unknown}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://releases.foo.example\"\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"configuration\"]\nvalidator = [\"foo\", \"guard\"]\n",
    )
    .expect("manifest should be written");
    let source = crate::run(&["doctor", path]);
    assert!(
        source.contains("configuration derivative requires an exact released binary"),
        "{source}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn attached() {
    let dir = seat("plumb-release-attachment-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.oci]\nregistry = \"git.perish.top\"\nimage = \"owner/name\"\naccount = \"PerishFire\"\n",
    )
    .expect("manifest should be written");
    let image = crate::run(&["doctor", path]);
    assert!(image.contains("publishes oci"), "{image}");
    assert!(!image.contains("the current Plumb refuses"), "{image}");

    std::fs::write(dir.join("plumb.toml"), "[release]\nskill = false\n")
        .expect("manifest should be written");
    let empty = crate::run(&["doctor", path]);
    assert!(empty.contains("the current Plumb refuses"), "{empty}");
    assert!(empty.contains("at least one attachment"), "{empty}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nskill = true\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let skill = crate::run(&["doctor", path]);
    assert!(skill.contains("requires a binary release"), "{skill}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn width() {
    let dir = seat("plumb-release-width");
    let path = dir.to_str().expect("path should be utf8");
    let names: Vec<String> = (1..=11).map(|index| format!("p{index}")).collect();
    for package in &names {
        std::fs::create_dir_all(dir.join("packages").join(package)).expect("module seat");
    }
    let declare = |packages: &str| {
        std::fs::write(
            dir.join("plumb.toml"),
            format!(
                "{BINARY}\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = {packages}\n"
            ),
        )
        .expect("manifest should be written");
    };

    let list = |count: usize| {
        let held: Vec<String> = names[..count]
            .iter()
            .map(|name| format!("\"@probe/{name}\""))
            .collect();
        format!("[{}]", held.join(", "))
    };

    declare(&list(1));
    let held = crate::run(&["doctor", path]);
    assert!(!held.contains("attachment declares"), "{held}");

    declare(&list(10));
    let edge = crate::run(&["doctor", path]);
    assert!(!edge.contains("and Plumb permits"), "{edge}");

    declare(&list(11));
    let wide = crate::run(&["doctor", path]);
    assert!(
        wide.contains("the npm attachment declares 11 packages and Plumb permits 10"),
        "{wide}"
    );
}

#[test]
fn settled() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    let release = commit(root, "release", Some(&base), &[]);
    let rejoin = commit(root, "rejoin", Some(&base), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert_eq!(identity::Seat(root).blind("plumb", Some(&release)), None);
}

#[test]
fn advanced() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    std::fs::write(root.join("source"), "changed").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let head = commit(root, "head", Some(&base), &[]);
    run(root, ["reset", "--hard", &head]);

    assert!(identity::Seat(root).blind("plumb", Some(&base)).is_some());
}

#[test]
fn drifted() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    let release = commit(root, "release", Some(&base), &[]);
    std::fs::write(root.join("source"), "drift").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let drift = commit(root, "drift", Some(&base), &[]);
    let rejoin = commit(root, "rejoin", Some(&drift), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert!(
        identity::Seat(root)
            .blind("plumb", Some(&release))
            .is_some()
    );
}

fn initialize(root: &std::path::Path) {
    run(root, ["init", "-q"]);
    run(root, ["config", "user.name", "Plumb"]);
    run(root, ["config", "user.email", "plumb@example.invalid"]);
    std::fs::write(root.join("source"), "base").expect("fixture file should be written");
    run(root, ["add", "source"]);
}

fn commit(root: &std::path::Path, message: &str, parent: Option<&str>, merges: &[&str]) -> String {
    let written = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("write-tree")
        .output()
        .expect("git should run");
    assert!(written.status.success(), "git write-tree should succeed");
    let tree = String::from_utf8_lossy(&written.stdout).trim().to_string();
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(["commit-tree", &tree]);
    if let Some(parent) = parent {
        command.args(["-p", parent]);
    }
    for merge in merges {
        command.args(["-p", merge]);
    }
    command.args(["-m", message]);
    let output = command.output().expect("git should run");
    assert!(output.status.success(), "git commit-tree should succeed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run<const N: usize>(root: &std::path::Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git should run");
    assert!(output.status.success(), "git command should succeed");
}
