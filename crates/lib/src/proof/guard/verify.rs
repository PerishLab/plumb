use std::process::Command;

use super::{Action, Authority, Descriptor, Expected, SCHEMA};

const COMMIT: &str = "1111111111111111111111111111111111111111";
const DEPOT: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const TREE: &str = "3333333333333333333333333333333333333333";

fn authority() -> Authority {
    Authority::fixture(format!("v0.0.0@{COMMIT}"), DEPOT)
}

fn proof() -> Descriptor {
    let mut proof = Descriptor {
        schema: SCHEMA.into(),
        repository: "PerishLab/probe".into(),
        tree: TREE.into(),
        plumb: format!("v0.0.0@{COMMIT}"),
        depot: DEPOT.into(),
        platform: crate::config::platform(),
        actions: vec![Action {
            name: "guard/test".into(),
            input: "4".repeat(64),
            world: "5".repeat(64),
        }],
        digest: String::new(),
    };
    proof.digest = proof.seal().expect("seal");
    proof
}

pub(super) fn seat() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("fixture");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(fixture.path())
        .status()
        .expect("git");
    assert!(status.success());
    let status = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/probe.git",
        ])
        .current_dir(fixture.path())
        .status()
        .expect("git");
    assert!(status.success());
    fixture
}

#[test]
fn unbound() {
    assert_eq!(
        Authority::released().err().as_deref(),
        Some("plumb-lib package carries no released authority")
    );
}

#[test]
fn package() {
    let manifest = format!(
        r#"[package]
name = "plumb"
version = "0.0.0"
[package.metadata.perish.guard]
schema = 1
depot = "{DEPOT}"
[package.metadata.perish.release]
schema = 1
repository = "PerishLab/plumb"
marker = "v0.0.0"
commit = "{COMMIT}"
tree = "{TREE}"
"#
    );
    assert_eq!(
        Authority::package(&manifest).expect("authority").producer(),
        format!("v0.0.0@{COMMIT}")
    );
}

#[test]
fn consumer() {
    let repository = seat();
    let proof = proof();
    let digest = proof.digest.clone();
    let expected = Expected::held(&proof);
    let verified = authority()
        .judge(repository.path(), proof, &expected)
        .expect("released authority should verify independently");
    assert_eq!(verified.descriptor().digest, digest);
}

#[test]
fn actions() {
    let repository = seat();
    let original = proof();
    let expected = Expected::held(&original);
    let mut changed = original;
    changed.actions[0].name = "guard/forged".into();
    changed.digest = changed.seal().expect("reseal");
    assert!(
        authority()
            .judge(repository.path(), changed, &expected)
            .expect_err("changed action must refuse")
            .contains("expected schema, tree, and digest")
    );
}

#[test]
fn producer() {
    let repository = seat();
    let proof = proof();
    let expected = Expected::held(&proof);
    assert!(
        Authority::fixture(format!("v0.0.0@{}", "6".repeat(40)), DEPOT)
            .judge(repository.path(), proof, &expected)
            .expect_err("producer mismatch must refuse")
            .contains("released authority")
    );
}

#[test]
fn depot() {
    let repository = seat();
    let proof = proof();
    let expected = Expected::held(&proof);
    assert!(
        Authority::fixture(format!("v0.0.0@{COMMIT}"), &"6".repeat(64))
            .judge(repository.path(), proof, &expected)
            .expect_err("Depot mismatch must refuse")
            .contains("released authority")
    );
}

#[test]
fn platform() {
    let repository = seat();
    let mut proof = proof();
    proof.platform = "forged-platform".into();
    proof.digest = proof.seal().expect("reseal");
    let expected = Expected::held(&proof);
    assert!(
        authority()
            .judge(repository.path(), proof, &expected)
            .expect_err("platform mismatch must refuse")
            .contains("forged-platform")
    );
}

#[test]
fn repository() {
    let repository = seat();
    let mut proof = proof();
    proof.repository = "Other/probe".into();
    proof.digest = proof.seal().expect("reseal");
    let expected = Expected::held(&proof);
    assert!(
        authority()
            .judge(repository.path(), proof, &expected)
            .expect_err("repository mismatch must refuse")
            .contains("Other/probe, not PerishLab/probe")
    );
}

#[test]
fn malformed() {
    let repository = seat();
    let mut proof = proof();
    proof.actions[0].input = "bad".into();
    let expected = Expected::held(&proof);
    assert!(
        authority()
            .judge(repository.path(), proof, &expected)
            .expect_err("malformed action must refuse")
            .contains("not a lowercase object digest")
    );
}

#[test]
fn expectation() {
    let repository = seat();
    let proof = proof();
    let expected = Expected::new(SCHEMA, "7".repeat(40), "8".repeat(64));
    assert!(
        authority()
            .judge(repository.path(), proof, &expected)
            .expect_err("different expected identity must refuse")
            .contains("expected schema, tree, and digest")
    );
}
