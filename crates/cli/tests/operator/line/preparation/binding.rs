use super::super::{command, datum, marker, stable, world};
use std::path::Path;
use std::process::Command;

#[test]
fn preflight() {
    let fixture = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let (url, calls) = world::serve(world::Court::Prepare(true, root.join("cut")), 1);
    let head = datum::lined(
        root,
        &format!("{url}/test/probe.git"),
        bare.path(),
        "release/v1.2.0",
    );
    let output = command::plumb(root, &["version", "prepare", "--version", "1.3.0"])
        .env("PLUMB_HOME", home.path())
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration preflight"));
    assert!(calls.lock().unwrap().is_empty());
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert!(
        git(
            bare.path(),
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads/release/v1.3.0"
            ]
        )
        .is_empty()
    );
}

#[test]
fn restored() {
    let fixture = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = fixture.path();
    stable::repo(root, "https://example.invalid/test/probe.git");
    git(root, &["config", "user.name", "Fixture"]);
    git(root, &["config", "user.email", "fixture@example.invalid"]);
    git(root, &["add", "-A"]);
    let tree = git(root, &["write-tree"]);
    let binary = Path::new(env!("CARGO_BIN_EXE_plumb"));
    let digest = plumb::depot::sha(&std::fs::read(binary).unwrap());
    let proof = plumb::guard::Descriptor::decode(&marker::proof(&tree, "test/probe"))
        .unwrap()
        .bootstrap(plumb::guard::Bootstrap {
            marker: plumb::depot::v3::Marker {
                name: "v9.9.9".into(),
                sha256: "1".repeat(64),
            },
            generation: "2".repeat(64),
            controller: digest,
            configuration: "3".repeat(64),
            validator: plumb::guard::Validator {
                version: "v9.9.9".into(),
                release: "4".repeat(64),
                artifact: "5".repeat(64),
            },
        })
        .unwrap();
    git(
        root,
        &[
            "commit",
            "-qm",
            &format!("Fixture\n\nPlumb-Guard-Proof: {}", proof.encode().unwrap()),
        ],
    );
    for args in [
        vec!["version", "prepare", "--version", "v1.0.0", "--dry-run"],
        vec!["release", "stamp", "--version", "v1.0.0", "--dry-run"],
        vec!["ship", "dispatch", "--marker", "v1.0.0", "--dry-run"],
        vec!["ship", "local", "--marker", "v1.0.0", "--dry-run"],
        vec![
            "depot",
            "configuration",
            "--marker",
            "v1.0.0",
            "--from",
            "media",
            "--stage",
            "--dry-run",
            "independent-target",
        ],
        vec![
            "depot",
            "skill",
            "--marker",
            "v1.0.0",
            "--from",
            "media",
            "--dry-run",
            "independent-target",
        ],
        vec![
            "depot",
            "changelog",
            "--marker",
            "v1.0.0",
            "--from",
            "media",
            "--dry-run",
            "independent-target",
        ],
    ] {
        let output = Command::new(binary)
            .args(&args)
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env_remove("PLUMB_GUARD_CONFIGURATION")
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(
            error.contains("cannot read installed candidate selection"),
            "{args:?}: {error}"
        );
    }
    let output = Command::new(binary)
        .args(["ship", "execute", "--request", "{}"])
        .current_dir(root)
        .env("PLUMB_HOME", home.path())
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("candidate selection"));
}

#[test]
fn projection() {
    let fixture = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let home = command::support::depot(&[]);
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, calls) = world::serve(world::Court::Prepare(true, cut.clone()), 7);
    datum::lined(
        root,
        &format!("{url}/test/probe.git"),
        bare.path(),
        "release/v1.2.0",
    );
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "invalid rust\n").unwrap();
    git(root, &["add", "Cargo.toml", "src"]);
    git(
        root,
        &["commit", "-qm", "Fixture\n\nPlumb-Guard-Proof: stale"],
    );
    git(root, &["push", "origin", "HEAD:refs/heads/main"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    std::fs::write(cut, &head).unwrap();
    let output = command::plumb(root, &["version", "prepare", "--version", "1.3.0"])
        .env("PLUMB_HOME", home.path())
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let requests = calls.lock().unwrap();
    assert!(!requests.is_empty());
    assert!(
        requests.iter().all(|request| request.starts_with("GET ")),
        "{requests:?}"
    );
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert!(
        git(
            bare.path(),
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads/release/v1.3.0"
            ]
        )
        .is_empty()
    );
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().into()
}
