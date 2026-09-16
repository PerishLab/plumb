use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub const CURL: &str = r#"#!/bin/sh
set -eu
destination=
status=false
for arg in "$@"; do url=$arg; done
while [ $# -gt 0 ]; do
  case "$1" in
    --output|-o) destination=$2; shift 2 ;;
    --write-out) status=true; shift 2 ;;
    *) shift ;;
  esac
done
case "$url" in
  */inventory.json)
    path="$FAKE_S3_ROOT/before.json"
    [ ! -f "$FAKE_CHART_REGISTRY" ] || path="$FAKE_S3_ROOT/after.json"
    ;;
  *) path="$FAKE_CHART_WORKLOAD" ;;
esac
if [ -n "$destination" ]; then cp "$path" "$destination"; else cat "$path"; fi
[ "$status" = false ] || printf '200'
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
    let invoke = || {
        let output = command()
            .args(["ship", "execute", "--request", &request.to_string()])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let failed = command()
        .env("PLUMB_TEST_REFUSE", "1")
        .args(["ship", "execute", "--request", &request.to_string()])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(!root.join("beta-registry.tgz").exists());
    let first = invoke();
    assert_eq!(first["schema"], "plumb.ship-result/v2");
    assert!(root.join("beta-registry.tgz").exists());
    let repeated = invoke();
    assert_eq!(first["evidence"], repeated["evidence"]);
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
        let mut command = fixture.controller();
        command
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test")
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
    assert_eq!(graph["schema"], "plumb.ship-dispatch/v1");
    let nodes = graph["graph"]["nodes"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .any(|node| node["id"] == format!("ship/produce.{target}"))
    );
    assert!(!root.path().join("dist").exists());
}
