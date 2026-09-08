use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

pub const CURL: &str = r#"#!/bin/sh
set -eu
destination=
for arg in "$@"; do url=$arg; done
while [ $# -gt 0 ]; do
  case "$1" in --output|-o) destination=$2; shift 2 ;; *) shift ;; esac
done
case "$url" in
  */inventory.json)
    path="$FAKE_S3_ROOT/before.json"
    [ ! -f "$FAKE_CHART_REGISTRY" ] || path="$FAKE_S3_ROOT/after.json"
    ;;
  *) path="$FAKE_CHART_WORKLOAD" ;;
esac
if [ -n "$destination" ]; then cp "$path" "$destination"; else cat "$path"; fi
"#;

pub fn run(root: &Path, command: &impl Fn() -> Command, request: &Value) {
    for (key, value) in [
        ("PLUMB_RELEASE_REGISTRY_ESCROW", "relative.env".to_string()),
        (
            "PLUMB_RELEASE_REGISTRY_ESCROW",
            root.join("missing.env").display().to_string(),
        ),
        (
            "PLUMB_RELEASE_REGISTRY_TOKEN",
            "Bearer conflicting-secret".to_string(),
        ),
    ] {
        let refused = command()
            .env(key, value)
            .args(["ship", "execute", "--request", &request.to_string()])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(error.contains("escrow"), "{error}");
        assert!(!error.contains("conflicting-secret"), "{error}");
        assert!(!root.join("beta-registry.tgz").exists());
    }
    let mut record = json!({
        "action":request["action"], "workload":request["keys"]["workload"],
        "proof":request["keys"]["proof"], "source":request["reuse"],
    });
    std::fs::write(
        root.join("before.json"),
        json!({
            "schema":"plumb.workflow-inventory/v1", "records":[record],
        })
        .to_string(),
    )
    .unwrap();
    record["publication"] = request["keys"]["publication"].clone();
    record["source"] = json!({"type":"url","source":"https://registry.example/owner/-/packages/container/probe/2.0.0-beta.1"});
    std::fs::write(
        root.join("after.json"),
        json!({
            "schema":"plumb.workflow-inventory/v1", "records":[record],
        })
        .to_string(),
    )
    .unwrap();
    let invoke = |dry| {
        let mut command = command();
        command.args(["ship", "local", "--marker", "v2.0.0-beta.1"]);
        if dry {
            command.arg("--dry-run");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "stdout:{} stderr:{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str::<Value>(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .last()
                .unwrap(),
        )
        .unwrap()
    };
    let dry = invoke(true);
    assert_eq!(dry["complete"], false);
    assert!(!root.join("home/tmp").exists());
    assert!(!root.join("beta-registry.tgz").exists());
    let blind = command()
        .env("PLUMB_RELEASE_REGISTRY_ESCROW", root.join("missing.env"))
        .env("PLUMB_WORKFLOW_INVENTORY_ESCROW", root.join("missing.env"))
        .args(["ship", "local", "--marker", "v2.0.0-beta.1", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        blind.status.success(),
        "{}",
        String::from_utf8_lossy(&blind.stderr)
    );
    let failed = command()
        .env("PLUMB_TEST_REFUSE", "1")
        .args(["ship", "local", "--marker", "v2.0.0-beta.1"])
        .output()
        .unwrap();
    assert!(
        !failed.status.success()
            && String::from_utf8_lossy(&failed.stderr).contains("repeat the same marker")
    );
    assert!(!root.join("beta-registry.tgz").exists());
    assert_eq!(std::fs::read_dir(root.join("home/tmp")).unwrap().count(), 0);
    let first = invoke(false);
    assert_eq!(first["schema"], "plumb.ship-local/v1");
    assert_eq!(first["complete"], true);
    assert_eq!(first["executed"].as_array().unwrap().len(), 1);
    assert!(!root.join("target/chart").exists());
    assert_eq!(std::fs::read_dir(root.join("home/tmp")).unwrap().count(), 0);
    let repeated = invoke(false);
    assert_eq!(repeated["complete"], true);
    assert_eq!(repeated["executed"], json!([]));
}

pub fn escrows(root: &Path, endpoint: &str) {
    let registry = root.join("registry.env");
    let workflow = root.join("workflow.env");
    std::fs::write(&registry, "REGISTRY_TOKEN_ID=1\nREGISTRY_TOKEN_NAME=probe-release-registry-v1\nREGISTRY_TOKEN_LAST_EIGHT=t-secret\nRELEASE_REGISTRY_TOKEN=fixture-test-secret\n").unwrap();
    std::fs::write(&workflow, format!("RELEASE_PUBLISH_S3_ACCESS_KEY=access\nRELEASE_PUBLISH_S3_SECRET_KEY=secret\nRELEASE_PUBLISH_S3_BUCKET=workflow\nRELEASE_PUBLISH_S3_ENDPOINT={endpoint}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [registry, workflow] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

#[test]
fn platform() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tools = root.path().join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    let fixture = crate::world::Fixture {
        root: root.path(),
        tools: &tools,
    };
    fixture.seed();
    let target = if cfg!(target_os = "macos") {
        "x86_64-unknown-linux-gnu"
    } else {
        "aarch64-apple-darwin"
    };
    let manifest = format!(
        "[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/compiler']\n[release]\nproduct='probe'\nauthority='https://releases.test'\nbinaries=['probe']\ntargets=['{target}']\n"
    );
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion='1.2.0'\n",
    )
    .unwrap();
    crate::marker::prepare(root.path(), home.path(), &manifest, "v1.2.0-beta.1");
    let command = || {
        let mut command = fixture.command();
        command
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test")
            .env(
                "PLUMB_WORKFLOW_INVENTORY_URL",
                "https://depot.test/inventory.json",
            )
            .env_remove("PLUMB_RELEASE_VERSION")
            .env_remove("PLUMB_RELEASE_CHANNEL")
            .env_remove("PLUMB_RELEASE_COMMIT")
            .args(["ship", "local", "--marker", "v1.2.0-beta.1"]);
        command
    };
    let dry = command().arg("--dry-run").output().unwrap();
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let graph: Value = serde_json::from_slice(&dry.stdout).unwrap();
    assert_eq!(graph["complete"], false);
    let refused = command().output().unwrap();
    let error = String::from_utf8_lossy(&refused.stderr);
    assert!(
        !refused.status.success() && error.contains("remains incomplete") && error.contains(target),
        "{error}"
    );
    assert!(!home.path().join("tmp").exists());
    std::fs::write(root.path().join("Cargo.toml"), "changed tracked source").unwrap();
    let dirty = command().arg("--dry-run").output().unwrap();
    assert!(
        !dirty.status.success()
            && String::from_utf8_lossy(&dirty.stderr).contains("tracked tree differs")
    );
}
