use std::path::Path;
use std::process::Command;

#[path = "../support.rs"]
mod support;

fn repository(name: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("repository fixture");
    let remote = format!("ssh://git@git.perish.top/PerishFire/{name}.git");
    for args in [vec!["init", "-q"], vec!["remote", "add", "origin", &remote]] {
        let done = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .status()
            .expect("git should run");
        assert!(done.success(), "fixture should become a repository");
    }
    root
}

fn publish(root: &Path, depot: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "depot",
            "skill",
            root.to_str().expect("repository path should be utf8"),
            "--marker",
            "invalid",
            "--from",
            root.to_str().expect("source path should be utf8"),
            "--dry-run",
        ])
        .env("PLUMB_DEPOT_SEAT", depot)
        .output()
        .expect("plumb should run");
    let mut held = String::from_utf8_lossy(&output.stdout).to_string();
    held.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), held)
}

#[test]
fn mapped() {
    let depot = support::depot(&[]);
    let ectropy = repository("ectropy");
    let (ok, held) = publish(ectropy.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(held.contains("invalid release marker vinvalid"), "{held}");
    assert!(!held.contains("plumb.toml"), "{held}");

    let unknown = repository("unknown");
    let (ok, held) = publish(unknown.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(
        held.contains(
            "perish.code product identity git.perish.top/PerishFire/unknown is absent from the Plumb depot"
        ),
        "{held}"
    );
}

#[test]
fn refused() {
    let rules = r#"
schema = "plumb.products/v1"

[[product]]
identity = "git.perish.top/PerishFire/ectropy"
name = "ectropy"
authority = "https://releases.ectropy.perish.uk"
derivatives = ["changelog"]
"#;
    let depot = support::depot(&[("rules/products.toml", rules)]);
    let ectropy = repository("ectropy");
    let (ok, held) = publish(ectropy.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(
        held.contains("perish.code product ectropy does not carry the skill derivative"),
        "{held}"
    );
}
