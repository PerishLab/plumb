#![cfg(unix)]

use sha2::{Digest, Sha256};
use std::process::Command;

#[test]
fn escrow() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("workflow.env");
    let workload = root.path().join("workload.tgz");
    std::fs::write(&workload, "workload").unwrap();
    let valid = "RELEASE_PUBLISH_S3_ACCESS_KEY=access\nRELEASE_PUBLISH_S3_SECRET_KEY=secret-fixture\nRELEASE_PUBLISH_S3_BUCKET=workflow\nRELEASE_PUBLISH_S3_ENDPOINT=https://s3.invalid\n";
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .args([
                "workflow",
                "record",
                "ship/chart",
                "--keys",
                &serde_json::json!({"workload":"1".repeat(64),"proof":"2".repeat(64)}).to_string(),
                "--workload",
                workload.to_str().unwrap(),
            ])
            .env_remove("PLUMB_WORKFLOW_INVENTORY_ACCESS")
            .env_remove("PLUMB_WORKFLOW_INVENTORY_SECRET")
            .env_remove("PLUMB_WORKFLOW_INVENTORY_SECRET_FILE")
            .env_remove("PLUMB_WORKFLOW_INVENTORY_BUCKET")
            .env_remove("PLUMB_WORKFLOW_INVENTORY_ENDPOINT")
            .env(
                "PLUMB_WORKFLOW_INVENTORY_URL",
                "https://inventory.invalid/inventory.json",
            )
            .env("PLUMB_WORKFLOW_INVENTORY_ESCROW", &path);
        command
    };
    for (body, mode) in [
        (valid.to_string(), 0o644),
        (
            format!("{valid}RELEASE_PUBLISH_S3_SECRET_KEY=duplicate-secret\n"),
            0o600,
        ),
        ("UNKNOWN=secret-fixture\n".to_string(), 0o600),
    ] {
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        let output = command().output().unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("escrow") || error.contains("mode 600"),
            "{error}"
        );
        assert!(
            !error.contains("secret-fixture") && !error.contains("duplicate-secret"),
            "{error}"
        );
    }
    std::fs::write(&path, valid).unwrap();
    let output = command()
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "conflicting")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("conflicts"));
}

#[test]
fn recorded() {
    let store = crate::support::Bucket::open(24);
    let root = tempfile::tempdir().expect("root");
    let workload = root.path().join("package.tgz");
    std::fs::write(&workload, "workload").expect("workload");
    let secret = root.path().join("secret");
    std::fs::write(&secret, "secret").unwrap();
    let keys = serde_json::json!({
        "workload": "1".repeat(64),
        "proof": "2".repeat(64),
        "publication": "3".repeat(64)
    })
    .to_string();
    let depot = serde_json::json!({
        "schema": "plumb.depot-worker/v1",
        "marker": "v1.0.0",
        "worker": "probe",
        "version": "worker-version"
    })
    .to_string();
    let record = || {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "workflow",
                "record",
                "ship/npm.cli",
                "--keys",
                &keys,
                "--workload",
                workload.to_str().expect("workload path"),
                "--publication",
                "https://registry.example/cli-1.0.0.tgz",
                "--depot",
                &depot,
            ])
            .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
            .env_remove("PLUMB_WORKFLOW_INVENTORY_SECRET")
            .env("PLUMB_WORKFLOW_INVENTORY_SECRET_FILE", &secret)
            .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
            .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", store.endpoint())
            .env(
                "PLUMB_WORKFLOW_INVENTORY_URL",
                "https://workflow.example/inventory.json",
            )
            .output()
            .expect("record")
    };
    let output = record();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let repeated = record();
    assert!(
        repeated.status.success(),
        "existing immutable records are idempotent: {}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    std::fs::write(&workload, "projected workload").expect("projected workload");
    let source = format!(
        "https://workflow.example/workloads/{:x}.tgz",
        Sha256::digest(b"workload")
    );
    let projected = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "workflow",
            "record",
            "ship/npm.cli",
            "--keys",
            &keys,
            "--workload",
            workload.to_str().expect("workload path"),
            "--reuse",
            &source,
            "--publication",
            "https://registry.example/cli-1.0.0.tgz",
            "--depot",
            &depot,
        ])
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
        .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
        .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", store.endpoint())
        .env(
            "PLUMB_WORKFLOW_INVENTORY_URL",
            "https://workflow.example/inventory.json",
        )
        .output()
        .expect("projected record");
    assert!(
        projected.status.success(),
        "reused source must survive projection"
    );
    let moved = keys
        .replace(&"2".repeat(64), &"4".repeat(64))
        .replace(&"3".repeat(64), &"5".repeat(64));
    std::fs::write(&workload, "moved workload").expect("moved workload");
    let compatible = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "workflow",
            "record",
            "ship/npm.cli",
            "--keys",
            &moved,
            "--workload",
            workload.to_str().expect("workload path"),
            "--publication",
            "https://registry.example/cli-1.0.0.tgz",
            "--depot",
            &depot,
        ])
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
        .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
        .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", store.endpoint())
        .env(
            "PLUMB_WORKFLOW_INVENTORY_URL",
            "https://workflow.example/inventory.json",
        )
        .output()
        .expect("compatible record");
    assert!(
        compatible.status.success(),
        "proof and source movement keep one content record: {}",
        String::from_utf8_lossy(&compatible.stderr)
    );
    let keys = store.keys();
    assert!(!keys.iter().any(|key| key == "inventory.json"));
    let records: Vec<serde_json::Value> = keys
        .iter()
        .filter(|key| key.starts_with("records/"))
        .map(|key| serde_json::from_slice(&store.read(key)).expect("record json"))
        .collect();
    assert_eq!(records.len(), 5);
    let workloads = records
        .iter()
        .filter(|record| record["source"]["type"] == "workload")
        .collect::<Vec<_>>();
    assert_eq!(workloads.len(), 3);
    assert!(workloads.iter().any(|record| record["proof"].is_null()));
    let source = records
        .iter()
        .find(|record| record["source"]["type"] == "workload")
        .and_then(|record| record["source"]["source"].as_str())
        .expect("workload source");
    assert!(source.starts_with("https://workflow.example/workloads/"));
    assert!(source.ends_with(".tgz"));
    let object = source
        .strip_prefix("https://workflow.example/")
        .expect("public prefix");
    assert!(store.has(object));
    let publication = records
        .iter()
        .find(|record| record["source"]["type"] == "url")
        .expect("publication record");
    assert_eq!(publication["depot"]["marker"], "v1.0.0");
    store.finish();
}
