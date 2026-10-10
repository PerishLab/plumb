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

#[test]
fn program() {
    if let Some(root) = std::env::var_os("PLUMB_PACKAGE_PROBE_ROOT") {
        let argv = vec!["plumb-package-probe".into(), "literal argument".into()];
        let output = super::process::run(std::path::Path::new(&root), "pnpm", &argv).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap().trim(),
            "literal argument"
        );
        println!("package-probe-verified");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("owned tools");
    std::fs::create_dir(&bin).unwrap();
    tool(&bin);
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "packages::tests::program", "--nocapture"])
        .env("PLUMB_PACKAGE_PROBE_ROOT", root.path())
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("package-probe-verified"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn family() {
    if let Some(root) = std::env::var_os("PLUMB_PACKAGE_FAMILY_ROOT") {
        let root = std::path::Path::new(&root);
        let lock = cargo::Lock(root);
        let wanted = |name: &str| Package {
            ecosystem: "cargo".into(),
            name: name.into(),
            version: "0.2.0".into(),
        };
        let family = [wanted("keel"), wanted("keel-gate")];
        lock.update(&family).unwrap();
        lock.verify(&family).unwrap();
        let log = std::fs::read_to_string(root.join("cargo.log")).unwrap();
        assert_eq!(log.lines().count(), 1, "{log}");
        assert!(!log.contains("--precise"), "{log}");
        std::fs::write(root.join("Cargo.lock"), held(&["plumb"])).unwrap();
        std::fs::remove_file(root.join("cargo.log")).unwrap();
        lock.update(&[wanted("plumb")]).unwrap();
        lock.verify(&[wanted("plumb")]).unwrap();
        let log = std::fs::read_to_string(root.join("cargo.log")).unwrap();
        assert_eq!(log.lines().count(), 1, "{log}");
        assert!(log.contains("--precise 0.2.0"), "{log}");
        println!("package-family-verified");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("owned tools");
    std::fs::create_dir(&bin).unwrap();
    pinned(&bin);
    std::fs::write(root.path().join("Cargo.lock"), held(&["keel", "keel-gate"])).unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "packages::tests::family", "--nocapture"])
        .env("PLUMB_PACKAGE_FAMILY_ROOT", root.path())
        .env("PATH", std::env::join_paths(paths).unwrap());
    for (key, _) in std::env::vars() {
        let ambient = key.starts_with("CARGO_") || key.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&key.as_str()) {
            command.env_remove(key);
        }
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}{stderr}");
    assert!(stdout.contains("package-family-verified"), "{stdout}");
}

#[cfg(unix)]
fn held(names: &[&str]) -> String {
    let mut lock = String::from("version=4\n");
    for name in names {
        lock.push_str(&format!(
            "[[package]]\nname=\"{name}\"\nversion=\"0.1.0\"\nsource=\"sparse+https://cargo.perish.uk/\"\n"
        ));
    }
    lock
}

#[cfg(unix)]
fn pinned(bin: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = bin.join("cargo");
    let script = r#"#!/bin/sh
printf '%s\n' "$*" >> cargo.log
old=$(grep -c 'version="0.1.0"' Cargo.lock)
case "$*" in
  *--precise*)
    if [ "$old" -gt 1 ]; then
      echo 'error: failed to select a version for the requirement `keel = "=0.1.0"`' >&2
      exit 101
    fi
    eval "set -- $*"
    while [ "$1" != --precise ]; do shift; done
    sed "s/0.1.0/$2/" Cargo.lock > Cargo.next ;;
  *) sed 's/0.1.0/0.2.0/' Cargo.lock > Cargo.next ;;
esac
mv Cargo.next Cargo.lock
"#;
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
fn tool(bin: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = bin.join("plumb-package-probe");
    std::fs::write(&path, "#!/bin/sh\nprintf '%s\\n' \"$1\"\n").unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(windows)]
fn tool(bin: &std::path::Path) {
    let path = bin.join("plumb-package-probe.cmd");
    std::fs::write(path, "@echo off\r\n@echo %~1\r\n").unwrap();
}
