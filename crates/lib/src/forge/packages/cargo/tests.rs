use super::{Lock, Package, SOURCE, graph, manifest};
use std::collections::{BTreeMap, BTreeSet};

const THIRD: &str = "registry+https://github.com/rust-lang/crates.io-index";

type Entry<'a> = (&'a str, &'a str, &'a [&'a str]);

fn lock(entries: &[Entry]) -> String {
    let mut text = String::from("version = 4\n");
    for (id, source, deps) in entries {
        let (name, version) = id.split_once(' ').unwrap();
        text.push_str(&format!(
            "\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"
        ));
        if !source.is_empty() {
            text.push_str(&format!("source = \"{source}\"\n"));
        }
        let deps = deps
            .iter()
            .map(|dep| format!(" \"{dep}\",\n"))
            .collect::<String>();
        if !deps.is_empty() {
            text.push_str(&format!("dependencies = [\n{deps}]\n"));
        }
    }
    text
}

fn world(keel: &str, errno: &str) -> String {
    lock(&[
        ("errno 0.3.14", THIRD, &[errno]),
        (&format!("keel {keel}"), SOURCE, &["errno", "keel-gate"]),
        (&format!("keel-gate {keel}"), SOURCE, &[]),
        ("probe 0.1.0", "", &["errno", "keel", "ring"]),
        ("ring 0.17.14", THIRD, &["windows-sys 0.52.0"]),
        ("windows-sys 0.52.0", THIRD, &[]),
        ("windows-sys 0.61.2", THIRD, &[]),
    ])
}

#[test]
fn without() {
    let mut before = world("0.1.0", "windows-sys 0.61.2");
    before.push_str(lock(&[("keel 0.1.0", THIRD, &[])]).trim_start_matches("version = 4\n"));
    let moves = BTreeSet::from([
        ("keel".to_string(), "0.1.0".to_string()),
        ("keel-gate".to_string(), "0.1.0".to_string()),
    ]);
    let text = graph::without(&before, &moves).unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    let held = value["package"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            format!(
                "{} {}",
                item["name"].as_str().unwrap(),
                item.get("source").is_some()
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        held,
        [
            "errno true",
            "probe false",
            "ring true",
            "windows-sys true",
            "windows-sys true",
            "keel true"
        ]
    );
    assert!(text.contains(&format!(
        "name = \"keel\"\nversion = \"0.1.0\"\nsource = \"{THIRD}\""
    )));
}

#[test]
fn minimal() {
    let before = world("0.1.0", "windows-sys 0.61.2");
    let after = world("0.2.0", "windows-sys 0.61.2");
    graph::forced(&before, &after).unwrap();
    graph::forced(&before, &before).unwrap();
}

#[test]
fn flipped() {
    let before = world("0.1.0", "windows-sys 0.61.2");
    let after = world("0.2.0", "windows-sys 0.52.0").replace(
        &format!(
            "\n[[package]]\nname = \"windows-sys\"\nversion = \"0.61.2\"\nsource = \"{THIRD}\"\n"
        ),
        "",
    );
    let error = graph::forced(&before, &after).unwrap_err();
    assert!(
        error.starts_with("unforced third-party Cargo lock change"),
        "{error}"
    );
    assert!(
        error.ends_with(": errno 0.3.14 moved windows-sys 0.61.2 to 0.52.0"),
        "{error}"
    );
}

#[test]
fn raised() {
    let before = world("0.1.0", "windows-sys 0.61.2");
    let after = lock(&[
        ("errno 0.3.20", THIRD, &["windows-sys 0.61.2"]),
        ("keel 0.3.0", SOURCE, &["errno", "keel-gate"]),
        ("keel-gate 0.3.0", SOURCE, &[]),
        ("probe 0.1.0", "", &["errno", "keel", "ring"]),
        ("ring 0.17.14", THIRD, &["windows-sys 0.52.0"]),
        ("windows-sys 0.52.0", THIRD, &[]),
        ("windows-sys 0.61.2", THIRD, &[]),
    ]);
    graph::forced(&before, &after).unwrap();
    let crossed = after.replace("0.3.20", "0.4.0");
    let error = graph::forced(&before, &crossed).unwrap_err();
    assert!(
        error.ends_with(": probe 0.1.0 moved errno 0.3.14 to 0.4.0"),
        "{error}"
    );
}

#[test]
fn stray() {
    let before = world("0.1.0", "windows-sys 0.61.2");
    let after = world("0.2.0", "windows-sys 0.61.2")
        .replace("\"keel\",\n \"ring\",", "\"keel\",\n \"stray\",")
        .replace(
            &format!("name = \"ring\"\nversion = \"0.17.14\"\nsource = \"{THIRD}\"\ndependencies = [\n \"windows-sys 0.52.0\",\n]\n"),
            &format!("name = \"stray\"\nversion = \"1.0.0\"\nsource = \"{THIRD}\"\n"),
        );
    let error = graph::forced(&before, &after).unwrap_err();
    for fault in [
        "probe 0.1.0 dropped ring 0.17.14",
        "probe 0.1.0 took stray 1.0.0",
        "ring 0.17.14 removed",
        "stray 1.0.0 added",
    ] {
        assert!(error.contains(fault), "{fault}: {error}");
    }
}

#[test]
fn pinned() {
    let text = "[workspace.dependencies]\nalias = { package = \"keel\", registry = \"perish\", version = \"0\", features = [\"gate\"] }\nlocal = { path = \"../local\", registry = \"perish\", version = \"0\" }\nplumb = { version = \"0\", registry = \"perish\" }\nerrno = \"0.3\"\n[target.'cfg(unix)'.dev-dependencies]\nkeel = { version = \"0\", registry = \"perish\" }\n";
    let versions = BTreeMap::from([
        ("keel".to_string(), "0.2.0".to_string()),
        ("local".to_string(), "9.0.0".to_string()),
    ]);
    let pinned = manifest::edit(text, &mut manifest::pin(&versions)).unwrap();
    assert_eq!(
        pinned,
        text.replace(
            "package = \"keel\", registry = \"perish\", version = \"0\"",
            "package = \"keel\", registry = \"perish\", version = \"=0.2.0\""
        )
        .replace(
            "keel = { version = \"0\", registry",
            "keel = { version = \"=0.2.0\", registry"
        )
    );
}

#[cfg(unix)]
#[test]
fn family() {
    if let Some(root) = std::env::var_os("PLUMB_PACKAGE_FAMILY_ROOT") {
        let root = std::path::Path::new(&root);
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        let wanted = |name: &str| Package {
            ecosystem: "cargo".into(),
            name: name.into(),
            version: "0.2.0".into(),
        };
        let family = [wanted("keel"), wanted("keel-gate")];
        let lock = Lock(root);
        let seats = ["Cargo.toml".to_string()];
        lock.update(&family, &seats).unwrap();
        lock.verify(&family).unwrap();
        let log = std::fs::read_to_string(root.join("cargo.log")).unwrap();
        assert_eq!(
            log,
            "metadata --format-version 1\nmetadata --format-version 1 --locked\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            manifest
        );
        let held = std::fs::read(root.join("Cargo.lock")).unwrap();
        lock.update(&family, &seats).unwrap();
        assert_eq!(std::fs::read(root.join("Cargo.lock")).unwrap(), held);
        assert_eq!(
            std::fs::read_to_string(root.join("cargo.log")).unwrap(),
            log
        );
        let base = world("0.1.0", "windows-sys 0.61.2");
        std::fs::write(root.join("Cargo.lock"), &base).unwrap();
        std::fs::write(root.join("next.lock"), world("0.2.0", "windows-sys 0.52.0")).unwrap();
        let error = lock.update(&family, &seats).unwrap_err();
        assert!(
            error.contains("errno 0.3.14 moved windows-sys 0.61.2 to 0.52.0"),
            "{error}"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            manifest
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Cargo.lock")).unwrap(),
            base
        );
        println!("package-family-verified");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("owned tools");
    std::fs::create_dir(&bin).unwrap();
    resolver(&bin);
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n\n[dependencies]\nkeel = { version = \"0\", registry = \"perish\" }\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("Cargo.lock"),
        world("0.1.0", "windows-sys 0.61.2"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("next.lock"),
        world("0.2.0", "windows-sys 0.61.2"),
    )
    .unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "packages::cargo::tests::family", "--nocapture"])
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
fn resolver(bin: &std::path::Path) {
    let script = r#"#!/bin/sh
case "$*" in
  *--locked*)
    printf '%s\n' "$*" >> cargo.log
    grep -q 'keel = { version = "0", registry' Cargo.toml || exit 101 ;;
  "metadata --format-version 1")
    printf '%s\n' "$*" >> cargo.log
    if grep -q '^name = "keel' Cargo.lock; then exit 102; fi
    grep -q 'keel = { version = "=0.2.0", registry' Cargo.toml || exit 103
    cp next.lock Cargo.lock ;;
  *) exit 104 ;;
esac
"#;
    plumb::testing::executable(&bin.join("cargo"), script);
}
