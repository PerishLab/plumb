use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const SEALED: [&str; 4] = ["cargo", "chart", "npm", "oci"];
const PRODUCT: &str = "[release]\nproduct = \"family\"\nauthority = \"https://example.invalid\"\nbinaries = [\"family\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n\n[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";
const ATTACHMENT: &str =
    "[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";
const WORKSPACE: &str = "[workspace]\nmembers = [\"crates/core\", \"crates/macro\", \"crates/helper\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.10.2\"\nedition = \"2024\"\nlicense = \"MIT\"\nrepository = \"https://example.invalid/family\"\n\n[workspace.dependencies]\ncore-alias = { package = \"family-core\", path = \"crates/core\", version = \"=0.10.2\" }\nhelper = { path = \"crates/helper\", version = \"=9.9.9\" }\nregistry-core = { package = \"family-core\", version = \"=0.10.2\", registry = \"perish\" }\n\n[workspace.dependencies.family-macro]\npath = \"crates/macro\"\nversion = \"=0.10.2\"\n";
const CORE: &str = "[package]\nname = \"family-core\"\nversion.workspace = true\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies]\nmacro-alias = { package = \"family-macro\", path = \"../macro\", version = \"=0.10.2\" }\nhelper = { path = \"../helper\", version = \"=9.9.9\" }\n\n[build-dependencies.family-macro]\npath = \"../macro\"\nversion = \"=0.10.2\"\n\n[dev-dependencies]\ncore-alias = { package = \"family-core\", path = \".\", version = \"=0.10.2\" }\n";
const MACRO: &str = "[package]\nname = \"family-macro\"\nversion = \"0.10.2\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies]\nhelper = { path = \"../helper\", version = \"=9.9.9\" }\n\n[dev-dependencies.family-core]\npath = \"../core\"\nversion = \"=0.10.2\"\n\n[dev-dependencies.core-alias]\npackage = \"family-core\"\npath = \"../core\"\nversion = \"=0.10.2\"\n";
const HELPER: &str = "[package]\nname = \"helper\"\nversion = \"9.9.9\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n";
const CARGO: &str = r#"#!/bin/sh
set -eu
if [ "$1" = metadata ]; then printf '%s\n' "{\"packages\":[{\"name\":\"family-core\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/core/Cargo.toml\",\"targets\":[]},{\"name\":\"family-macro\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/macro/Cargo.toml\",\"targets\":[]},{\"name\":\"helper\",\"version\":\"9.9.9\",\"manifest_path\":\"$PWD/crates/helper/Cargo.toml\",\"targets\":[]}],\"target_directory\":\"$PWD/target\"}"; exit 0; fi
[ "$(grep -c 'version = \"=0.10.2-beta.1\"' Cargo.toml)" -eq 2 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/core/Cargo.toml)" -eq 3 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/macro/Cargo.toml)" -eq 2 ] || { printf '%s\n' 'error: failed to select a version for requirement =0.10.2; candidate 0.10.2-beta.1 did not match' >&2; exit 101; }
grep -F 'version = "0.10.2-beta.1"' Cargo.toml >/dev/null && grep -F 'version = "0.10.2-beta.1"' crates/macro/Cargo.toml >/dev/null
grep -F 'helper = { path = "crates/helper", version = "=9.9.9" }' Cargo.toml >/dev/null && grep -F 'registry-core = { package = "family-core", version = "=0.10.2", registry = "perish" }' Cargo.toml >/dev/null && grep -F 'helper = { path = "../helper", version = "=9.9.9" }' crates/core/Cargo.toml >/dev/null && grep -F 'version = "9.9.9"' crates/helper/Cargo.toml >/dev/null
printf '0.10.2-beta.1\n' > cargo-observed
name=""
prev=""
for arg in "$@"; do
  if [ "$prev" = --package ]; then name="$arg"; fi
  prev="$arg"
done
mkdir -p "target/package/$name-0.10.2-beta.1"
printf 'version = "0.10.2-beta.1"\n' > "target/package/$name-0.10.2-beta.1/Cargo.toml"
tar -czf "target/package/$name-0.10.2-beta.1.crate" -C target/package "$name-0.10.2-beta.1"
"#;

#[test]
fn cargo() {
    let root = tempfile::tempdir().expect("Cargo fixture");
    let path = root.path();
    for package in ["core", "macro", "helper"] {
        std::fs::create_dir_all(path.join("crates").join(package)).expect("crate root");
    }
    for (seat, text) in [
        ("plumb.toml", ATTACHMENT),
        ("Cargo.toml", WORKSPACE),
        ("crates/core/Cargo.toml", CORE),
        ("crates/macro/Cargo.toml", MACRO),
        ("crates/helper/Cargo.toml", HELPER),
        ("cargo", CARGO),
    ] {
        std::fs::write(path.join(seat), text).expect("Cargo fixture file");
    }
    std::fs::set_permissions(path.join("cargo"), std::fs::Permissions::from_mode(0o755))
        .expect("Cargo mode");
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        let env = format!(
            "{}:{}",
            path.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command.env("PATH", env).env("PLUMB_RELEASE_ROOT", path);
        command
    };
    let output = command()
        .args(["ship", "cargo", "rehearse"])
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env_remove("PLUMB_RELEASE_REGISTRY_TOKEN")
        .output()
        .expect("plumb should run");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success() && error.contains("PLUMB_RELEASE_REGISTRY_TOKEN is required"));
    let output = command()
        .args(["ship", "cargo", "rehearse"])
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(path.join("cargo-observed")).expect("observation"),
        "0.10.2-beta.1\n"
    );
}

#[test]
fn sealed() {
    let root = tempfile::tempdir().expect("capsule fixture");
    let path = root.path();
    std::fs::write(path.join("plumb.toml"), PRODUCT).expect("attachment");
    let ship = |adaptor: &str, version: &str| {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["ship", adaptor, "publish"])
            .env("PLUMB_RELEASE_ROOT", path)
            .env("PLUMB_RELEASE_OUTPUT", ".plumb-release")
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "secret")
            .output()
            .expect("plumb should run")
    };
    for adaptor in SEALED {
        let bare = ship(adaptor, "v0.10.2-beta.1");
        let missing = String::from_utf8_lossy(&bare.stderr).to_string();
        assert!(
            !bare.status.success() && missing.contains("capsule.json"),
            "{adaptor}: {missing}"
        );
    }

    let out = path.join(".plumb-release");
    std::fs::create_dir_all(&out).expect("release output");
    let body = b"{}";
    let record = out.join("seal.json");
    std::fs::write(&record, body).expect("seal");
    let served = format!("file://{}", record.display());
    let digest = format!("{:x}", Sha256::digest(body));
    let capsule = format!(
        concat!(
            r#"{{"schema":1,"product":"family","channel":"beta","#,
            r#""releaseVersion":"v0.10.2-beta.1","authority":"https://example.invalid","#,
            r#""objects":[],"seal":{{"source":"seal.json","key":"v1/seal.json","#,
            r#""remote":{{"name":"seal.json","mime":"application/json","sha256":"{}","#,
            r#""size":{},"url":"{}"}}}},"#,
            r#""roots":[],"pointer":null}}"#
        ),
        digest,
        body.len(),
        served
    );
    std::fs::write(out.join("capsule.json"), &capsule).expect("capsule");

    for adaptor in SEALED {
        let drift = ship(adaptor, "v0.10.2-beta.2");
        let refused = String::from_utf8_lossy(&drift.stderr).to_string();
        assert!(
            !drift.status.success()
                && refused.contains(
                    "capsule seals v0.10.2-beta.1 while the projection carries v0.10.2-beta.2"
                ),
            "{adaptor}: {refused}"
        );
    }

    for (adaptor, medium) in [("chart", "chart"), ("npm", "module"), ("oci", "image")] {
        let absent = ship(adaptor, "v0.10.2-beta.1");
        let said = String::from_utf8_lossy(&absent.stdout).to_string();
        assert!(
            absent.status.success() && said.contains(&format!("has no {medium} attachment")),
            "{adaptor}: {said}"
        );
    }

    std::fs::create_dir_all(path.join("packages/family")).expect("module seat");
    std::fs::write(
        path.join("plumb.toml"),
        format!(
            "{PRODUCT}\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = [\"@family/family\"]\n"
        ),
    )
    .expect("attachment");
    let malformed = ship("npm", "v0.10.2-beta.1");
    let refused = String::from_utf8_lossy(&malformed.stderr).to_string();
    assert!(
        !malformed.status.success() && refused.contains("must be a Cargo Bearer credential"),
        "{refused}"
    );

    let elsewhere = out.join("elsewhere.json");
    std::fs::write(&elsewhere, b"{\"other\":true}").expect("served");
    std::fs::write(
        out.join("capsule.json"),
        capsule.replace(&served, &format!("file://{}", elsewhere.display())),
    )
    .expect("capsule");
    let drifted = ship("npm", "v0.10.2-beta.1");
    let said = String::from_utf8_lossy(&drifted.stderr).to_string();
    assert!(
        !drifted.status.success() && said.contains("public object drift"),
        "{said}"
    );
}

#[test]
fn unsealed() {
    let root = tempfile::tempdir().expect("attachment fixture");
    let path = root.path();
    std::fs::write(path.join("plumb.toml"), ATTACHMENT).expect("attachment");
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "cargo", "publish"])
        .env("PLUMB_RELEASE_ROOT", path)
        .env_remove("PLUMB_RELEASE_OUTPUT")
        .env_remove("PLUMB_RELEASE_CAPSULE")
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("plumb should run");
    let reached = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!reached.contains("PLUMB_RELEASE_OUTPUT"), "{reached}");
    assert!(!reached.contains("capsule"), "{reached}");
    assert!(reached.contains("cargo metadata failed"), "{reached}");
}

#[test]
fn settled() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    std::fs::create_dir_all(root.join("packages/held")).expect("package root");
    std::fs::write(
        root.join("plumb.toml"),
        "[release.npm]\nregistry = \"https://registry.invalid\"\npackages = [\"held\"]\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("packages/held/package.json"),
        "{\"name\":\"held\"}\n",
    )
    .expect("package");
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.test"],
        vec!["add", "-A"],
        vec!["commit", "-qm", "seed"],
    ] {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git");
        assert!(status.success());
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "npm", "pack"])
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .output()
        .expect("plumb should run");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !text.contains("unchanged since"),
        "a product with no authority holds no baseline, so nothing is settled: {text}"
    );
}
