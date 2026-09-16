use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[path = "exact.rs"]
mod exact;
#[path = "runseal.rs"]
mod runseal;

const ATTACHMENT: &str = "[release]\nproduct = \"family\"\nauthority = \"https://releases.family.test\"\n\n[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";
const WORKSPACE: &str = "[workspace]\nmembers = [\"crates/core\", \"crates/macro\", \"crates/helper\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.10.2\"\nedition = \"2024\"\nlicense = \"MIT\"\nrepository = \"https://example.invalid/family\"\n\n[workspace.dependencies]\ncore-alias = { package = \"family-core\", path = \"crates/core\", version = \"=0.10.2\" }\nhelper = { path = \"crates/helper\", version = \"=9.9.9\" }\nregistry-core = { package = \"family-core\", version = \"=0.10.2\", registry = \"perish\" }\n\n[workspace.dependencies.family-macro]\npath = \"crates/macro\"\nversion = \"=0.10.2\"\n";
const CORE: &str = "[package]\nname = \"family-core\"\nversion.workspace = true\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies]\nmacro-alias = { package = \"family-macro\", path = \"../macro\", version = \"=0.10.2\" }\nhelper = { path = \"../helper\", version = \"=9.9.9\" }\n\n[build-dependencies.family-macro]\npath = \"../macro\"\nversion = \"=0.10.2\"\n\n[dev-dependencies]\ncore-alias = { package = \"family-core\", path = \".\", version = \"=0.10.2\" }\n";
const MACRO: &str = "[package]\nname = \"family-macro\"\nversion = \"0.10.2\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies]\nhelper = { path = \"../helper\", version = \"=9.9.9\" }\n\n[dev-dependencies.family-core]\npath = \"../core\"\nversion = \"=0.10.2\"\n\n[dev-dependencies.core-alias]\npackage = \"family-core\"\npath = \"../core\"\nversion = \"=0.10.2\"\n";
const HELPER: &str = "[package]\nname = \"helper\"\nversion = \"9.9.9\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n";
const CARGO: &str = r#"#!/bin/sh
set -eu
if [ "$1" = metadata ]; then printf '%s\n' "{\"packages\":[{\"name\":\"family-core\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/core/Cargo.toml\",\"targets\":[]},{\"name\":\"family-macro\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/macro/Cargo.toml\",\"targets\":[]},{\"name\":\"helper\",\"version\":\"9.9.9\",\"manifest_path\":\"$PWD/crates/helper/Cargo.toml\",\"targets\":[]}],\"target_directory\":\"$PWD/target\"}"; exit 0; fi
printf '%s\n' "$*" >> "$FAKE_CARGO_ROOT/cargo-calls"
[ "$(grep -c 'version = \"=0.10.2-beta.1\"' Cargo.toml)" -eq 3 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/core/Cargo.toml)" -eq 4 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/macro/Cargo.toml)" -eq 3 ] || { printf '%s\n' 'error: failed to select a version for requirement =0.10.2; candidate 0.10.2-beta.1 did not match' >&2; exit 101; }
grep -F 'version = "0.10.2-beta.1"' Cargo.toml >/dev/null && grep -F 'version = "0.10.2-beta.1"' crates/macro/Cargo.toml >/dev/null
grep -F 'helper = { path = "crates/helper", version = "=0.10.2-beta.1" }' Cargo.toml >/dev/null && grep -F 'registry-core = { package = "family-core", version = "=0.10.2", registry = "perish" }' Cargo.toml >/dev/null && grep -F 'helper = { path = "../helper", version = "=0.10.2-beta.1" }' crates/core/Cargo.toml >/dev/null && grep -F 'version = "0.10.2-beta.1"' crates/helper/Cargo.toml >/dev/null
printf '0.10.2-beta.1\n' > cargo-observed
name=""
prev=""
for arg in "$@"; do
  if [ "$prev" = --package ]; then name="$arg"; fi
  prev="$arg"
done
mkdir -p "target/package/$name-0.10.2-beta.1"
printf 'version = "0.10.2-beta.1"\n' > "target/package/$name-0.10.2-beta.1/Cargo.toml"
archive="target/package/$name-0.10.2-beta.1.crate"
if [ ! -f "$archive" ]; then tar -czf "$archive" -C target/package "$name-0.10.2-beta.1"; fi
if [ "$1" = publish ]; then
  mkdir -p "$FAKE_CARGO_ROOT/target/package"
  if [ "$PWD" != "$FAKE_CARGO_ROOT" ]; then cp "$archive" "$FAKE_CARGO_ROOT/$archive"; fi
  touch "$FAKE_CARGO_ROOT/published-$name"
fi
"#;
#[test]
fn cargo() {
    let root = tempfile::tempdir().expect("Cargo fixture");
    let home = crate::support::depot(&[]);
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
        ("curl", exact::CURL),
        ("aws", exact::AWS),
    ] {
        std::fs::write(path.join(seat), text).expect("Cargo fixture file");
    }
    for tool in ["cargo", "curl", "aws"] {
        std::fs::set_permissions(path.join(tool), std::fs::Permissions::from_mode(0o755))
            .expect("tool mode");
    }
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        let env = format!(
            "{}:{}",
            path.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .env("PLUMB_HOME", home.path())
            .env("PATH", env)
            .env("FAKE_CARGO_ROOT", path)
            .env("PLUMB_RELEASE_ROOT", path);
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

    exact::prove(path);
}

#[test]
fn retired() {
    for args in [
        vec!["ship", "cargo", "publish"],
        vec!["ship", "npm", "publish"],
        vec!["ship", "npm", "exact"],
        vec!["ship", "chart", "publish"],
        vec!["ship", "chart", "exact"],
        vec!["ship", "oci", "exact"],
        vec!["ship", "oci", "publish"],
        vec!["ship", "binary", "publish"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
    }
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
    let home = tempfile::tempdir().unwrap();
    let binding = crate::marker::prepare(
        root,
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.npm]\nregistry='https://registry.invalid'\npackages=['held']\n",
        "v1.2.0-beta.1",
    );
    let exact = |package: &str, reuse: &str| {
        let request = serde_json::json!({
            "schema":"plumb.ship-request/v3", "marker":"v1.2.0-beta.1", "configuration":binding["configuration"], "profile":binding["profile"],
            "action":"ship/npm.held",
            "operation":{"type":"npm","package":package},
            "reuse":serde_json::from_str::<serde_json::Value>(reuse).unwrap(),
        });
        std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test")
            .args(["ship", "execute", "--request", &request.to_string()])
            .env("PLUMB_RELEASE_ROOT", root)
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .output()
            .expect("plumb should run")
    };
    let unknown = exact("other", r#"{"type":"none","source":""}"#);
    assert!(
        !unknown.status.success()
            && String::from_utf8_lossy(&unknown.stderr)
                .contains("not a declared marker-bound Ship request")
    );
    let published = exact(
        "held",
        r#"{"type":"url","source":"https://registry.invalid/held"}"#,
    );
    assert!(
        !published.status.success()
            && String::from_utf8_lossy(&published.stderr)
                .contains("publication requires its declared production workload")
    );
}
