use super::{Package, cargo, manifest, npm, registry};
use std::collections::BTreeSet;

#[test]
fn stable() {
    let index = b"{\"vers\":\"0.1.0\"}\n{\"vers\":\"0.3.0-rc.1\"}\n{\"vers\":\"0.9.0\",\"yanked\":true}\n{\"vers\":\"0.2.0\"}\n";
    assert_eq!(registry::cargo(index).unwrap(), "0.2.0");
    assert!(registry::cargo(b"not-json").is_err());
    assert!(registry::cargo(b"{\"vers\":\"0.3.0-rc.1\"}").is_err());
}

#[test]
fn manifests() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Cargo.toml");
    let text = "[workspace.dependencies]\nalias = { package = \"plumb\", registry = \"perish\", version = \"=0.1.0\", features = [\"delivery\"] }\nlocal = { path = \"../local\", registry = \"perish\", version = \"=0.0.0\" }\nthird = \"1\"\n[target.'cfg(windows)'.dependencies]\nkeel = { version = \"0.1\", registry = \"perish\" }\n[package.metadata.dependencies]\nfiction = { version = \"1\", registry = \"perish\" }\n";
    std::fs::write(&file, text).unwrap();
    let mut names = BTreeSet::new();
    manifest::read(dir.path(), "Cargo.toml", &mut names)
        .unwrap()
        .unwrap()
        .write(dir.path())
        .unwrap();
    let result = std::fs::read_to_string(&file).unwrap();
    assert!(result.contains("version = \"0\", features"));
    assert!(result.contains("version = \"=0.0.0\""));
    assert!(result.contains("fiction = { version = \"1\""));
    assert_eq!(
        names,
        BTreeSet::from([
            ("cargo".into(), "plumb".into()),
            ("cargo".into(), "keel".into())
        ])
    );
    assert!(
        manifest::read(dir.path(), "Cargo.toml", &mut names)
            .unwrap()
            .is_none()
    );
}

#[test]
fn json() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("package.json");
    std::fs::write(&file,r#"{"dependencies":{"@perishlab/design":"0.3.2","third":"1","@perishlab/local":"workspace:*"},"scripts":{"build":"unchanged"}}"#).unwrap();
    let mut names = BTreeSet::new();
    manifest::read(dir.path(), "package.json", &mut names)
        .unwrap()
        .unwrap()
        .write(dir.path())
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(value["dependencies"]["@perishlab/design"], "0");
    assert_eq!(value["dependencies"]["third"], "1");
    assert_eq!(value["dependencies"]["@perishlab/local"], "workspace:*");
    assert_eq!(value["scripts"]["build"], "unchanged");
    assert_eq!(names.len(), 1);
    assert!(
        manifest::read(dir.path(), "package.json", &mut names)
            .unwrap()
            .is_none()
    );
}

#[test]
fn changed() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Cargo.toml");
    std::fs::write(
        &file,
        "[dependencies]\nplumb={version=\"0.1\",registry=\"perish\"}\n",
    )
    .unwrap();
    let edit = manifest::read(dir.path(), "Cargo.toml", &mut BTreeSet::new())
        .unwrap()
        .unwrap();
    std::fs::write(&file, "someone else's valuable edit").unwrap();
    assert!(edit.write(dir.path()).is_err());
    assert_eq!(
        std::fs::read_to_string(file).unwrap(),
        "someone else's valuable edit"
    );
}

#[test]
fn crates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.lock"),"version=4\n[[package]]\nname=\"plumb\"\nversion=\"0.1.0\"\nsource=\"sparse+https://cargo.perish.uk/\"\n[[package]]\nname=\"plumb\"\nversion=\"1.0.0\"\nsource=\"registry+https://github.com/rust-lang/crates.io-index\"\n").unwrap();
    let lock = cargo::Lock(dir.path());
    let mut names = BTreeSet::new();
    lock.names(&mut names).unwrap();
    assert_eq!(names.len(), 1);
    let mut package = Package {
        ecosystem: "cargo".into(),
        name: "plumb".into(),
        version: "0.1.0".into(),
    };
    lock.verify(&[package.clone()]).unwrap();
    package.version = "0.2.0".into();
    assert!(lock.verify(&[package]).is_err());
}

#[test]
fn packages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pnpm-lock.yaml");
    std::fs::write(&path,"lockfileVersion: '9.0'\npackages:\n  '@perishlab/design@0.3.2': {}\n  third@1.0.0: {}\nsnapshots:\n  '@perishlab/design@0.3.2(svelte@5.0.0)': {}\n").unwrap();
    let lock = npm::Lock(dir.path());
    let mut names = BTreeSet::new();
    lock.names(&mut names).unwrap();
    assert_eq!(names.len(), 1);
    lock.verify(&[Package {
        ecosystem: "npm".into(),
        name: "@perishlab/design".into(),
        version: "0.3.2".into(),
    }])
    .unwrap();
    std::fs::write(path, "lockfileVersion: 'unknown'\n").unwrap();
    assert!(lock.names(&mut names).is_err());
}
