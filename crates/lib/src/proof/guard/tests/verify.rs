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
        resolution: None,
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
fn pinned() {
    let depot = Authority::pinned().expect("plumb-lib pins its Depot mark");
    assert_eq!(depot.len(), 64);
    let manifest = |schema: u64| {
        format!(
            "[package]\nname = \"plumb\"\nversion = \"0.0.0\"\n\
             [package.metadata.perish.guard]\nschema = {schema}\ndepot = \"{DEPOT}\"\n"
        )
    };
    let pin = |text: &str| {
        super::authority::perish(text).and_then(|held| super::authority::pin(held.guard))
    };
    assert_eq!(pin(&manifest(1)).as_deref(), Ok(DEPOT));
    assert_eq!(
        pin(&manifest(2)).err().as_deref(),
        Some("plumb-lib Guard authority schema 2 is not 1")
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

#[test]
fn resolutions() {
    let original = proof();
    assert!(
        serde_json::to_value(&original)
            .unwrap()
            .get("resolution")
            .is_none()
    );
    let resolution = crate::packages::Resolution {
        context: "ci-latest".into(),
        tree: "a".repeat(40),
        packages: vec![crate::packages::Package {
            ecosystem: "cargo".into(),
            name: "plumb".into(),
            version: "0.1.0".into(),
        }],
    };
    let resolved = original.clone().resolved(resolution).unwrap();
    assert_ne!(original.digest, resolved.digest);
    assert_eq!(
        Descriptor::decode(&resolved.encode().unwrap()).unwrap(),
        resolved
    );
    let mut changed = resolved.clone();
    changed.resolution.as_mut().unwrap().packages[0].version = "0.2.0".into();
    assert!(changed.validate().is_err());
    let mut invalid = resolved.resolution.unwrap();
    invalid.context = "pretend-local".into();
    assert!(original.resolved(invalid).is_err());
}
