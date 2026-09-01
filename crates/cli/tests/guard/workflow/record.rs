#![cfg(unix)]

use std::process::Command;

#[test]
fn recorded() {
    let store = crate::support::Bucket::open(6);
    let root = tempfile::tempdir().expect("root");
    let workload = root.path().join("package.tgz");
    std::fs::write(&workload, "workload").expect("workload");
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
            .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
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
        "an existing inventory uses its ETag: {}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    let inventory: serde_json::Value =
        serde_json::from_slice(&store.read("inventory.json")).expect("inventory json");
    assert_eq!(inventory["schema"], "plumb.workflow-inventory/v1");
    assert_eq!(inventory["records"].as_array().expect("records").len(), 2);
    let source = inventory["records"]
        .as_array()
        .expect("records")
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
    let publication = inventory["records"]
        .as_array()
        .expect("records")
        .iter()
        .find(|record| record["source"]["type"] == "url")
        .expect("publication record");
    assert_eq!(publication["depot"]["marker"], "v1.0.0");
    store.finish();
}
