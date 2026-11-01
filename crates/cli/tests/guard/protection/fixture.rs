use serde_json::Value;
use sha2::Digest;
use std::process::{Command, Output};

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) home: tempfile::TempDir,
    tools: tempfile::TempDir,
    host: String,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let version = Command::new("rustc").arg("-vV").output().unwrap();
        let host = String::from_utf8(version.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
            .unwrap();
        let held = Self {
            root: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
            tools: tempfile::tempdir().unwrap(),
            host,
        };
        held.git(&["init", "-q", "-b", "topic"]);
        held.git(&[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/probe.git",
        ]);
        held.put("Cargo.toml", "[package]\nname=\"probe\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[features]\nextra=[]\n");
        held.put(
            "Cargo.lock",
            "version=4\n[[package]]\nname=\"probe\"\nversion=\"0.0.0\"\n",
        );
        held.put("src/lib.rs", "pub fn answer() -> u8 {\n    42\n}\n");
        held.put(
            "ectropy.toml",
            "[[ban]]\nsyntax=\"lock\"\npaths=[\"src/**\"]\n",
        );
        let version = Command::new("ectropy").arg("--version").output().unwrap();
        assert!(version.status.success());
        let version = String::from_utf8(version.stdout).unwrap();
        let body = format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '%s' {}; exit 0; fi\nprintf x >> {}\nif [ -f {} ]; then exit 79; fi\nprintf '%s\\n' clean\n",
            quote(&version),
            quote(held.home.path().join("calls").to_str().unwrap()),
            quote(held.home.path().join("fail").to_str().unwrap())
        );
        super::super::support::executable(&held.tools.path().join("ectropy"), &body);
        held.policy(false, false);
        held
    }

    pub(super) fn put(&self, name: &str, value: &str) {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }

    pub(super) fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }

    pub(super) fn policy(&self, enforce: bool, ectropy: bool) {
        let scan = if ectropy {
            "ectropy=[\"ectropy.toml\"]\n"
        } else {
            ""
        };
        self.put("plumb.toml", &format!("[workflow.hash.guard]\nrust=[\"Cargo.toml\",\"Cargo.lock\",\"src\"]\n{scan}[guard.lock]\nenforce={enforce}\ntarget={:?}\nfeatures=[\"default\",\"all-features\"]\n", self.host));
        self.git(&["add", "."]);
    }

    pub(super) fn run(&self) -> Output {
        let path = std::env::var_os("PATH").unwrap();
        let path = std::env::join_paths(
            std::iter::once(self.tools.path().to_path_buf()).chain(std::env::split_paths(&path)),
        )
        .unwrap();
        super::super::support::plumb()
            .args(["guard", ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PATH", path)
            .env("PLUMB_GUARD_CACHE_RESERVE", "0")
            .env_remove("SCCACHE_DIR")
            .env_remove("RUSTC_WRAPPER")
            .output()
            .unwrap()
    }

    pub(super) fn reviews(&self) {
        let mut entries = std::collections::BTreeMap::new();
        let digest = format!(
            "{:x}",
            sha2::Sha256::digest(std::fs::read(self.root.path().join("protocol.txt")).unwrap())
        );
        for all in [false, true] {
            let mut command = super::super::support::plumb();
            command
                .args(["lock", "resolve", ".", "--json"])
                .current_dir(self.root.path())
                .env("PLUMB_HOME", self.home.path());
            if all {
                command.arg("--all-features");
            }
            let report = success(command.output().unwrap());
            for finding in report["messages"]["findings"].as_array().unwrap() {
                let reference = finding["reference"]["digest"].as_str().unwrap();
                let evidence = serde_json::json!({"protocol.txt":digest});
                let value = serde_json::json!({"reference":reference,"role":"interoperability","protocol":"fixture protocol","hold":"reviewed fixture interval","release":"reviewed fixture cleanup","verification":"source protocol test, not formal qualification","evidence":evidence});
                entries.insert(reference.to_owned(), value);
            }
        }
        let file = self.root.path().join("plumb.toml");
        let mut document: toml::Table = std::fs::read_to_string(file).unwrap().parse().unwrap();
        let reviews =
            serde_json::from_value(serde_json::json!(entries.into_values().collect::<Vec<_>>()))
                .unwrap();
        document["guard"]["lock"]
            .as_table_mut()
            .unwrap()
            .insert("review".into(), reviews);
        self.put("plumb.toml", &toml::to_string(&document).unwrap());
        self.git(&["add", "."]);
    }

    pub(super) fn calls(&self) -> usize {
        std::fs::read(self.home.path().join("calls"))
            .unwrap_or_default()
            .len()
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(super) fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(super) fn rejected(output: Output, reason: &str) {
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(reason),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(super) fn action<'a>(proof: &'a Value, name: &str) -> &'a Value {
    proof["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["name"] == name)
        .unwrap()
}
