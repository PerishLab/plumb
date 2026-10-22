use super::{Action, Copy, Transfer};

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
