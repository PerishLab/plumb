use super::precommit::Repo;
use super::support;
use serde_json::Value;
use std::path::Path;
use std::process::Output;

const THIRD: &str = "registry+https://github.com/rust-lang/crates.io-index";

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
    tools: tempfile::TempDir,
}

impl Fixture {
    fn new(direct: bool, offered: &[&str]) -> Self {
        Self::shaped(direct, false, offered)
    }

    fn shaped(direct: bool, kept: bool, offered: &[&str]) -> Self {
        let tools = tempfile::tempdir().unwrap();
        Self::sources(tools.path());
        Self::offer(tools.path(), offered);
        support::executable(
            &tools.path().join("curl"),
            "#!/bin/sh\nfor last; do :; done\ncat \"$(dirname \"$0\")/index/${last##*/}\"\n",
        );
        let root = tempfile::tempdir().unwrap();
        let seat = root.path();
        Repo::git(seat, &["init", "-q", "-b", "topic"]);
        Repo::git(seat, &["config", "user.name", "Plumb Test"]);
        Repo::git(seat, &["config", "user.email", "plumb@example.invalid"]);
        let origin = "https://git.example.invalid/Example/probe.git";
        Repo::git(seat, &["remote", "add", "origin", origin]);
        std::fs::create_dir_all(seat.join("src")).unwrap();
        std::fs::create_dir_all(seat.join(".cargo")).unwrap();
        std::fs::write(
            seat.join("src/lib.rs"),
            "pub fn answer() -> u8 {\n    42\n}\n",
        )
        .unwrap();
        let errno = if direct { "errno = \"0.3\"\n" } else { "" };
        let wire = if kept {
            "mio = \"1\"\nwire = { version = \"0\", registry = \"perish\" }\n"
        } else {
            ""
        };
        std::fs::write(
            seat.join("Cargo.toml"),
            format!("[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{errno}keel = {{ version = \"0\", registry = \"perish\" }}\nring = \"0.17\"\n{wire}"),
        )
        .unwrap();
        let base = if kept {
            Self::kept(direct)
        } else {
            Self::base(direct)
        };
        std::fs::write(seat.join("Cargo.lock"), base).unwrap();
        let place = |name: &str| toml::Value::String(tools.path().join(name).display().to_string());
        std::fs::write(
            seat.join(".cargo/config.toml"),
            format!("[registries.perish]\nindex = \"sparse+https://cargo.perish.uk/\"\n\n[source.crates-io]\nreplace-with = \"third\"\n\n[source.perish]\nregistry = \"sparse+https://cargo.perish.uk/\"\nreplace-with = \"first\"\n\n[source.third]\ndirectory = {}\n\n[source.first]\ndirectory = {}\n", place("third"), place("first")),
        )
        .unwrap();
        std::fs::write(
            seat.join("plumb.toml"),
            "[workflow.hash.guard]\nrust = [\".cargo\", \"Cargo.toml\", \"Cargo.lock\", \"src\"]\n",
        )
        .unwrap();
        Repo::git(seat, &["add", "."]);
        Repo::git(seat, &["commit", "-q", "-m", "fixture"]);
        Self {
            root,
            home: support::home(),
            tools,
        }
    }

    fn sources(tools: &Path) {
        let first = "registry-index = \"sparse+https://cargo.perish.uk/\"";
        for (name, version, deps) in [
            ("windows-sys", "0.52.0", String::new()),
            ("windows-sys", "0.61.2", String::new()),
            ("errno", "0.3.14", "windows-sys = \">=0.52, <0.62\"".into()),
            ("errno", "0.3.20", "windows-sys = \">=0.52, <0.62\"".into()),
            ("ring", "0.17.14", "windows-sys = \"0.52\"".into()),
            ("mio", "1.2.4", "windows-sys = \"0.61\"".into()),
        ] {
            Self::package(&tools.join("third"), (name, version), &deps);
        }
        for (version, errno) in [("0.1.0", "0.3"), ("0.2.0", "0.3"), ("0.3.0", "0.3.20")] {
            let deps =
                format!("errno = \"{errno}\"\nkeel-gate = {{ version = \"={version}\", {first} }}");
            Self::package(&tools.join("first"), ("keel", version), &deps);
            Self::package(&tools.join("first"), ("keel-gate", version), "");
        }
        Self::package(&tools.join("first"), ("wire", "0.1.0"), "errno = \"0.3\"");
    }

    fn package(seat: &Path, (name, version): (&str, &str), deps: &str) {
        let dir = seat.join(format!("{name}-{version}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2021\"\n\n[dependencies]\n{deps}\n"),
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "").unwrap();
        std::fs::write(
            dir.join(".cargo-checksum.json"),
            "{\"files\":{},\"package\":null}",
        )
        .unwrap();
    }

    fn offer(tools: &Path, versions: &[&str]) {
        std::fs::create_dir_all(tools.join("index")).unwrap();
        let mut keel = String::new();
        let mut gate = String::new();
        for version in versions {
            let errno = if *version == "0.3.0" {
                "^0.3.20"
            } else {
                "^0.3"
            };
            keel.push_str(&format!("{{\"name\":\"keel\",\"vers\":\"{version}\",\"deps\":[{{\"name\":\"keel-gate\",\"req\":\"={version}\",\"kind\":\"normal\"}},{{\"name\":\"errno\",\"req\":\"{errno}\",\"kind\":\"normal\",\"registry\":\"https://github.com/rust-lang/crates.io-index\"}}]}}\n"));
            gate.push_str(&format!(
                "{{\"name\":\"keel-gate\",\"vers\":\"{version}\",\"deps\":[]}}\n"
            ));
        }
        std::fs::write(tools.join("index/keel"), keel).unwrap();
        std::fs::write(tools.join("index/keel-gate"), gate).unwrap();
        std::fs::write(
            tools.join("index/wire"),
            "{\"name\":\"wire\",\"vers\":\"0.1.0\",\"deps\":[{\"name\":\"errno\",\"req\":\"^0.3\",\"kind\":\"normal\",\"registry\":\"https://github.com/rust-lang/crates.io-index\"}]}\n",
        )
        .unwrap();
    }

    fn base(direct: bool) -> String {
        let probe = if direct { " \"errno\",\n" } else { "" };
        format!(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"errno\"\nversion = \"0.3.14\"\nsource = \"{THIRD}\"\ndependencies = [\n \"windows-sys 0.61.2\",\n]\n\n[[package]]\nname = \"keel\"\nversion = \"0.1.0\"\nsource = \"sparse+https://cargo.perish.uk/\"\ndependencies = [\n \"errno\",\n \"keel-gate\",\n]\n\n[[package]]\nname = \"keel-gate\"\nversion = \"0.1.0\"\nsource = \"sparse+https://cargo.perish.uk/\"\n\n[[package]]\nname = \"probe\"\nversion = \"0.1.0\"\ndependencies = [\n{probe} \"keel\",\n \"ring\",\n]\n\n[[package]]\nname = \"ring\"\nversion = \"0.17.14\"\nsource = \"{THIRD}\"\ndependencies = [\n \"windows-sys 0.52.0\",\n]\n\n[[package]]\nname = \"windows-sys\"\nversion = \"0.52.0\"\nsource = \"{THIRD}\"\n\n[[package]]\nname = \"windows-sys\"\nversion = \"0.61.2\"\nsource = \"{THIRD}\"\n"
        )
    }

    fn kept(direct: bool) -> String {
        let first = "source = \"sparse+https://cargo.perish.uk/\"";
        let mio = format!(
            "[[package]]\nname = \"mio\"\nversion = \"1.2.4\"\nsource = \"{THIRD}\"\ndependencies = [\n \"windows-sys 0.61.2\",\n]\n\n"
        );
        let wire = format!(
            "\n[[package]]\nname = \"wire\"\nversion = \"0.1.0\"\n{first}\ndependencies = [\n \"errno\",\n]\n"
        );
        let mut lock = Self::base(direct)
            .replace(
                "dependencies = [\n \"windows-sys 0.61.2\",\n]",
                "dependencies = [\n \"windows-sys 0.52.0\",\n]",
            )
            .replace(
                "[[package]]\nname = \"probe\"",
                &format!("{mio}[[package]]\nname = \"probe\""),
            )
            .replace(
                " \"keel\",\n \"ring\",\n]",
                " \"keel\",\n \"mio\",\n \"ring\",\n \"wire\",\n]",
            );
        lock.push_str(&wire);
        lock
    }

    fn plumb(&self, verb: &str) -> Output {
        let mut paths = vec![self.tools.path().to_path_buf()];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        support::plumb()
            .args([verb, ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PLUMB_GUARD_CACHE_RESERVE", "0")
            .env("CARGO_TARGET_DIR", self.root.path().join("unowned"))
            .env("PLUMB_GUARD_STRENGTH", "full")
            .env("PLUMB_GUARD_BOUNDARY", "head")
            .env("PATH", std::env::join_paths(paths).unwrap())
            .output()
            .unwrap()
    }

    fn lock(&self) -> String {
        std::fs::read_to_string(self.root.path().join("Cargo.lock")).unwrap()
    }

    fn git(&self, args: &[&str]) -> String {
        String::from_utf8(Repo::git(self.root.path(), args).stdout).unwrap()
    }
}

fn report(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn moved(lock: &str, version: &str) -> String {
    let first = "source = \"sparse+https://cargo.perish.uk/\"";
    ["keel", "keel-gate"]
        .iter()
        .fold(lock.to_string(), |lock, name| {
            lock.replace(
                &format!("name = \"{name}\"\nversion = \"0.1.0\"\n{first}"),
                &format!("name = \"{name}\"\nversion = \"{version}\"\n{first}"),
            )
        })
}

#[test]
fn minimal() {
    let fixture = Fixture::new(true, &["0.1.0", "0.2.0"]);
    let before = fixture.lock();
    let resolution = report(&fixture.plumb("lift"));
    assert_eq!(resolution["packages"][0]["name"], "keel");
    assert_eq!(resolution["packages"][0]["version"], "0.2.0");
    assert_eq!(resolution["packages"][1]["version"], "0.2.0");
    assert_eq!(fixture.lock(), moved(&before, "0.2.0"));
    assert_eq!(fixture.git(&["status", "--porcelain"]), " M Cargo.lock\n");
    Repo::git(fixture.root.path(), &["commit", "-q", "-am", "lift"]);
    let lifted = fixture.lock();
    report(&fixture.plumb("lift"));
    assert_eq!(fixture.lock(), lifted);
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
}

#[test]
fn kept() {
    let fixture = Fixture::shaped(true, true, &["0.1.0", "0.2.0"]);
    let before = fixture.lock();
    assert!(
        before.contains("name = \"wire\"\nversion = \"0.1.0\""),
        "{before}"
    );
    let resolution = report(&fixture.plumb("lift"));
    assert_eq!(resolution["packages"][0]["version"], "0.2.0");
    assert_eq!(fixture.lock(), moved(&before, "0.2.0"));
    assert_eq!(fixture.git(&["status", "--porcelain"]), " M Cargo.lock\n");
}

#[test]
fn raised() {
    let fixture = Fixture::new(false, &["0.1.0", "0.2.0", "0.3.0"]);
    let before = fixture.lock();
    let resolution = report(&fixture.plumb("lift"));
    assert_eq!(resolution["packages"][0]["version"], "0.3.0");
    let lock = fixture.lock();
    assert!(
        lock.contains("name = \"errno\"\nversion = \"0.3.20\""),
        "{lock}"
    );
    assert!(!lock.contains("0.3.14"), "{lock}");
    assert_ne!(lock, moved(&before, "0.3.0"));
    assert!(lock.contains("name = \"errno\"\nversion = \"0.3.20\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\n \"windows-sys\",\n]"), "{lock}");
    assert!(!lock.contains("0.61.2"), "{lock}");
}

#[test]
fn shared() {
    let fixture = Fixture::new(true, &["0.1.0", "0.2.0", "0.3.0"]);
    let before = fixture.lock();
    let output = fixture.plumb("lift");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("failed to select a version for `errno`"),
        "{stderr}"
    );
    assert_eq!(fixture.lock(), before);
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
}

#[test]
fn guard() {
    let fixture = Fixture::new(true, &["0.1.0", "0.2.0"]);
    let before = fixture.lock();
    let evidence = report(&fixture.plumb("guard"));
    let resolution = &evidence["guard"]["resolution"];
    assert_eq!(resolution["context"], "ci-latest");
    assert_eq!(resolution["packages"][0]["version"], "0.2.0");
    assert_eq!(resolution["packages"][1]["version"], "0.2.0");
    assert_eq!(fixture.lock(), before);
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
}
