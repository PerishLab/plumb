use super::native;
use super::{Action, Copy, Transfer};
use crate::command::cache::parcel::{Parcel, Plan};

#[test]
fn existing() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("target");
    std::fs::create_dir_all(source.join("nested")).unwrap();
    std::fs::create_dir(&target).unwrap();
    std::fs::write(source.join("held"), "incoming").unwrap();
    std::fs::write(source.join("nested/new"), "new").unwrap();
    std::fs::write(target.join("held"), "existing").unwrap();
    assert_eq!(
        Copy {
            source,
            target: target.clone()
        }
        .merge()
        .unwrap(),
        1
    );
    assert_eq!(
        std::fs::read_to_string(target.join("held")).unwrap(),
        "existing"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("nested/new")).unwrap(),
        "new"
    );
}

#[test]
fn foreign() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("foreign");
    std::fs::write(&target, "owned elsewhere").unwrap();
    let parcel = target.join("material.tar.zst");
    let transfer = Transfer {
        root: root.path(),
        home: root.path(),
        context: None,
        parcel: &parcel,
        family: "unused",
        identity: "unused",
    };
    assert!(
        transfer
            .run(Action::Inspect)
            .err()
            .unwrap()
            .contains("foreign slot")
    );
    assert!(
        Copy {
            source: root.path().join("absent"),
            target: target.clone()
        }
        .merge()
        .unwrap_err()
        .contains("foreign directory")
    );
    assert_eq!(std::fs::read_to_string(target).unwrap(), "owned elsewhere");
}

#[test]
#[cfg(unix)]
fn symlink() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("target");
    let foreign = root.path().join("foreign");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    std::fs::write(&foreign, "owned elsewhere").unwrap();
    std::fs::write(source.join("entry"), "incoming").unwrap();
    std::os::unix::fs::symlink(&foreign, target.join("entry")).unwrap();
    assert!(
        Copy {
            source: source.clone(),
            target: target.clone()
        }
        .merge()
        .unwrap_err()
        .contains("foreign entry")
    );
    assert_eq!(
        std::fs::read_to_string(&foreign).unwrap(),
        "owned elsewhere"
    );
    std::fs::remove_file(target.join("entry")).unwrap();
    std::fs::remove_dir(&target).unwrap();
    std::os::unix::fs::symlink(&source, &target).unwrap();
    assert!(
        Copy {
            source,
            target: target.clone()
        }
        .merge()
        .unwrap_err()
        .contains("foreign directory")
    );
    assert!(
        std::fs::symlink_metadata(target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn producers() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    let compiler = root.path().join("source");
    std::fs::create_dir(&compiler).unwrap();
    std::fs::write(compiler.join("entry"), "compiled").unwrap();
    let parcel = root.path().join("slot/material.tar.zst");
    std::fs::create_dir(parcel.parent().unwrap()).unwrap();
    Parcel(&parcel)
        .write(Plan {
            identity: "world;producer-a".into(),
            cargo: None,
            compiler: Some(compiler),
            timing: None,
        })
        .unwrap();
    for (name, family, identity) in [
        ("exact", "world;", "world;producer-a"),
        ("other", "world;", "world;producer-b"),
        ("foreign", "elsewhere;", "elsewhere;producer-a"),
    ] {
        let reused = name != "foreign";
        let home = root.path().join(name);
        std::fs::create_dir(&home).unwrap();
        let receipt = Transfer {
            root: root.path(),
            home: &home,
            context: None,
            parcel: &parcel,
            family,
            identity,
        }
        .run(Action::Restore { fetch: false })
        .unwrap();
        assert!(!receipt.cargo, "{name}");
        assert_eq!(receipt.compiler, reused, "{name}");
        assert_eq!(home.join("cache/compiler/entry").exists(), reused, "{name}");
    }
}

fn skipped(vars: &[(&str, &str)]) -> Option<&'static str> {
    native::decide(|name| {
        vars.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    })
    .unwrap()
    .map(|skip| skip.reason())
}

#[test]
fn follow() {
    assert_eq!(
        skipped(&[("GITHUB_HEAD_REF", "auto/259")]),
        Some("follow-branch")
    );
    assert_eq!(
        skipped(&[("GITHUB_HEAD_REF", ""), ("GITHUB_REF", "refs/heads/auto/7")]),
        Some("follow-branch")
    );
    assert_eq!(
        skipped(&[
            ("GITHUB_HEAD_REF", "auto/1"),
            ("PLUMB_GUARD_NATIVE", "auto")
        ]),
        Some("follow-branch")
    );
}

#[test]
fn ordinary() {
    assert_eq!(skipped(&[]), None);
    assert_eq!(skipped(&[("GITHUB_HEAD_REF", "feature/690")]), None);
    assert_eq!(skipped(&[("GITHUB_HEAD_REF", "task/auto/3")]), None);
    assert_eq!(
        skipped(&[
            ("GITHUB_HEAD_REF", "feature/9"),
            ("GITHUB_REF", "refs/pull/9/merge")
        ]),
        None
    );
    assert_eq!(skipped(&[("GITHUB_REF", "refs/heads/main")]), None);
    assert_eq!(skipped(&[("GITHUB_REF", "refs/tags/auto/1")]), None);
}

#[test]
fn explicit() {
    assert_eq!(
        skipped(&[
            ("GITHUB_HEAD_REF", "auto/259"),
            ("PLUMB_GUARD_NATIVE", "keep")
        ]),
        None
    );
    assert_eq!(
        skipped(&[
            ("GITHUB_HEAD_REF", "feature/1"),
            ("PLUMB_GUARD_NATIVE", "skip")
        ]),
        Some("requested")
    );
    let error = native::decide(|name| (name == "PLUMB_GUARD_NATIVE").then(|| "maybe".to_string()))
        .err()
        .unwrap();
    assert!(error.contains("must be auto, keep or skip"));
}

#[test]
fn compiler() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    let home = root.path().join("home");
    std::fs::create_dir_all(home.join("cache/compiler")).unwrap();
    std::fs::write(home.join("cache/compiler/entry"), "compiled").unwrap();
    let context = crate::command::cache::transport::Context {
        image: format!("sha256:{}", "a".repeat(64)),
    };
    let parcel = root.path().join("slot/material.tar.zst");
    std::fs::create_dir(parcel.parent().unwrap()).unwrap();
    let skip =
        native::decide(|name| (name == "GITHUB_HEAD_REF").then(|| "auto/5".to_string())).unwrap();
    assert!(skip.is_some());
    let receipt = Transfer {
        root: root.path(),
        home: &home,
        context: Some(&context),
        parcel: &parcel,
        family: "world;",
        identity: "world;producer",
    }
    .pack(skip)
    .unwrap();
    assert!(!receipt.cargo);
    assert!(receipt.compiler);
    assert!(parcel.exists());
}
