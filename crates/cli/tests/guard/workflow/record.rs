#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt as _, path::Path, process::Command};

#[test]
fn recorded() {
    let root = std::env::temp_dir().join(format!("plumb-workflow-record-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let bin = root.join("bin");
    let store = root.join("store");
    fs::create_dir_all(&bin).expect("bin");
    fs::create_dir_all(&store).expect("store");
    let aws = bin.join("aws");
    fs::write(
        &aws,
        r#"#!/bin/sh
set -eu
operation=
bucket=
key=
body=
output=
while test "$#" -gt 0; do
  case "$1" in
    get-object|put-object) operation=$1 ;;
    --bucket) shift; bucket=$1 ;;
    --key) shift; key=$1 ;;
    --body) shift; body=$1 ;;
    --endpoint-url|--content-type|--cache-control|--if-match|--if-none-match) shift ;;
    --no-cli-pager|s3api) ;;
    *) test "$operation" != get-object || output=$1 ;;
  esac
  shift
done
target="$PLUMB_TEST_STORE/$key"
case "$operation" in
  get-object)
    if test ! -f "$target"; then
      echo NoSuchKey >&2
      exit 1
    fi
    cp "$target" "$output"
    echo '{"ETag":"held"}'
    ;;
  put-object)
    mkdir -p "$(dirname "$target")"
    cp "$body" "$target"
    echo '{}'
    ;;
  *) exit 2 ;;
esac
"#,
    )
    .expect("aws");
    let mut mode = fs::metadata(&aws).expect("aws metadata").permissions();
    mode.set_mode(0o755);
    fs::set_permissions(&aws, mode).expect("aws mode");
    let workload = root.join("package.tgz");
    fs::write(&workload, "workload").expect("workload");
    let keys = serde_json::json!({
        "workload": "1".repeat(64),
        "proof": "2".repeat(64),
        "publication": "3".repeat(64)
    })
    .to_string();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").expect("PATH"));
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
            ])
            .env("PATH", &path)
            .env("PLUMB_TEST_STORE", &store)
            .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
            .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
            .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
            .env(
                "PLUMB_WORKFLOW_INVENTORY_ENDPOINT",
                "https://account.r2.cloudflarestorage.com",
            )
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
        "an existing inventory reads the AWS ETag dialect: {}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    let inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(store.join("inventory.json")).expect("inventory"))
            .expect("inventory json");
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
    assert!(exists(&store, source));
}

fn exists(store: &Path, source: &str) -> bool {
    let key = source
        .strip_prefix("https://workflow.example/")
        .expect("public prefix");
    store.join(key).is_file()
}
