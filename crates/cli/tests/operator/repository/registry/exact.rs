use std::path::Path;
use std::process::Command;

pub const CURL: &str = r#"#!/bin/sh
set -eu
cd "$FAKE_CARGO_ROOT"
case "$*" in
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
    let inventory = crate::support::Bucket::open(4);
    let request = serde_json::json!({
        "schema": "plumb.ship-request/v2",
        "action": "ship/cargo",
        "projections": [],
        "roots": ["Cargo.toml", "Cargo.lock", "crates"],
        "operation": { "type": "cargo" },
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
