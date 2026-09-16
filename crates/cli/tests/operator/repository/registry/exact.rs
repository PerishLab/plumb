use std::path::Path;
use std::process::Command;

pub const CURL: &str = r#"#!/bin/sh
set -eu
cd "$FAKE_CARGO_ROOT"
case "$*" in
  *workflow.example/v2/blobs/sha256/*) cat "$FAKE_CARGO_ROOT/reuse.tgz"; exit 0 ;;
  *family-macro*) package=family-macro ;;
  *family-core*) package=family-core ;;
  *) printf '\n404'; exit 0 ;;
esac
if [ ! -f "published-$package" ]; then printf '\n404'; exit 0; fi
archive="target/package/$package-0.10.2-beta.1.crate"
checksum=$(sha256sum "$archive" | cut -d' ' -f1)
printf '{"vers":"0.10.2-beta.1","cksum":"%s"}\n200' "$checksum"
"#;

pub const AWS: &str = r#"#!/bin/sh
case "$*" in
  *get-object*) printf '%s\n' NoSuchKey >&2; exit 1 ;;
  *) printf '{}\n' ;;
esac
"#;

pub fn prove(path: &Path) {
    let home = tempfile::tempdir().unwrap();
    let binding = crate::marker::prepare(path, home.path(), super::ATTACHMENT, "v0.10.2-beta.1");
    let reuse = std::fs::File::create(path.join("reuse.tgz")).expect("reuse workload");
    let reuse = flate2::write::GzEncoder::new(reuse, flate2::Compression::default());
    let mut reuse = tar::Builder::new(reuse);
    for name in [
        "Cargo.toml",
        "crates/core/Cargo.toml",
        "crates/macro/Cargo.toml",
        "crates/helper/Cargo.toml",
    ] {
        let body = std::fs::read(path.join(name)).unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        reuse
            .append_data(&mut header, name, body.as_slice())
            .expect("carried crate");
    }
    reuse
        .into_inner()
        .expect("finish reuse archive")
        .finish()
        .expect("finish reuse workload");
    let request = serde_json::json!({
        "schema": "plumb.ship-request/v3",
        "marker": "v0.10.2-beta.1",
        "configuration": binding["configuration"],
        "profile": binding["profile"],
        "action": "ship/cargo",
        "operation": { "type": "cargo" },
        "reuse": {
            "type": "workload",
            "source": format!("https://workflow.example/v2/blobs/sha256/{}",
                plumb::depot::sha(&std::fs::read(path.join("reuse.tgz")).unwrap()))
        }
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .current_dir(path)
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .args(["ship", "execute", "--request", &request])
        .env(
            "PATH",
            format!(
                "{}:{}",
                path.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("FAKE_CARGO_ROOT", path)
        .env("PLUMB_RELEASE_ROOT", path)
        .env("PLUMB_RELEASE_CHANNEL", "beta")
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("exact Cargo request");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).expect("exact result");
    assert_eq!(result["schema"], "plumb.ship-result/v2");
    assert_eq!(result["evidence"]["schema"], "plumb.ship-resource/v1");
    assert_eq!(result["evidence"]["action"], "ship/cargo");
    let calls = std::fs::read_to_string(path.join("cargo-calls")).expect("Cargo calls");
    assert!(calls.contains("package --registry perish"), "{calls}");
    assert!(calls.contains("publish --registry perish"), "{calls}");
    assert!(!calls.contains("--dry-run"), "{calls}");
    assert!(std::path::Path::new(result["projection"]["workload"].as_str().unwrap()).is_file());
}
