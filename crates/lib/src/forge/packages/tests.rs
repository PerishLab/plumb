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
fn tool(bin: &std::path::Path) {
    let body = concat!("#!/bin/sh\n", "printf '%s\n' \"$1\"\n");
    plumb::testing::executable(&bin.join("plumb-package-probe"), body);
}

#[cfg(windows)]
fn tool(bin: &std::path::Path) {
    let body = concat!("@echo off\r\n", "@echo %~1\r\n");
    std::fs::write(bin.join("plumb-package-probe.cmd"), body).unwrap();
}

#[test]
fn compressed() {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/pl/um/plumb", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut gzip = false;
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            let header = line.to_ascii_lowercase();
            gzip |= header.starts_with("accept-encoding:") && header.contains("gzip");
            line.clear();
        }
        let gzipped: &[u8] = &[
            31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 171, 86, 42, 75, 45, 42, 86, 178, 82, 50, 208, 51,
            210, 51, 80, 170, 229, 2, 0, 44, 44, 102, 188, 17, 0, 0, 0,
        ];
        let (encoding, body) = match gzip {
            true => ("Content-Encoding: gzip\r\n", gzipped),
            false => ("", &b"{\"vers\":\"0.1.0\"}\n"[..]),
        };
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\n{encoding}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
    });
    let argv = registry::probe(&url);
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(registry::cargo(&output.stdout).unwrap(), "0.2.0");
}
