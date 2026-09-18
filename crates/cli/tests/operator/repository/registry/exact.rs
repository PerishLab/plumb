use std::path::Path;
use std::process::Command;

pub const CURL: &str = r#"#!/bin/sh
set -eu
cd "$FAKE_CARGO_ROOT"
case "$*" in
  *workflow.example/workload.tgz*) cat "$FAKE_CARGO_ROOT/reuse.tgz"; exit 0 ;;
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
    let inventory = crate::support::Bucket::open(3);
    let reuse = std::fs::File::create(path.join("reuse.tgz")).expect("reuse workload");
    let reuse = flate2::write::GzEncoder::new(reuse, flate2::Compression::default());
    let mut reuse = tar::Builder::new(reuse);
    for package in ["family-macro", "family-core"] {
        let body = b"prior marker crate";
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        reuse
            .append_data(
                &mut header,
                format!("{package}-0.10.2-beta.0.crate"),
                body.as_slice(),
            )
            .expect("carried crate");
    }
    reuse
        .into_inner()
        .expect("finish reuse archive")
        .finish()
        .expect("finish reuse workload");
    let request = serde_json::json!({
        "schema": "plumb.ship-request/v2",
        "action": "ship/cargo",
        "projections": [],
        "roots": ["Cargo.toml", "Cargo.lock", "crates"],
        "operation": { "type": "cargo" },
        "reuse": {
            "type": "workload",
            "source": "https://workflow.example/workload.tgz"
        },
        "keys": {
            "workload": "1".repeat(64),
            "proof": "2".repeat(64),
            "publication": "3".repeat(64)
        }
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
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
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
        .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
        .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", inventory.endpoint())
        .env(
            "PLUMB_WORKFLOW_INVENTORY_URL",
            "https://workflow.example/inventory.json",
        )
        .output()
        .expect("exact Cargo request");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).expect("exact result");
    assert_eq!(result["result"]["type"], "url");
    let calls = std::fs::read_to_string(path.join("cargo-calls")).expect("Cargo calls");
    assert!(calls.contains("package --registry perish"), "{calls}");
    assert!(calls.contains("publish --registry perish"), "{calls}");
    assert!(!calls.contains("--dry-run"), "{calls}");
    assert!(path.join("target/cargo/family-cargo.tar.gz").is_file());
    inventory.finish();
}
