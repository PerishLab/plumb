use std::path::Path;
use std::process::Command;

#[test]
fn exact() {
    let fixture = tempfile::tempdir().expect("worker fixture");
    let root = fixture.path();
    super::seed(root);
    std::fs::write(
        root.join("plumb.toml"),
        "[release.cfworker]\naccount = \"account\"\ndomain = \"site.test\"\n",
    )
    .expect("release manifest");
    let request = |reuse: serde_json::Value| {
        serde_json::json!({
            "schema": "plumb.ship-request/v1",
            "action": "ship/cfworker",
            "projections": [],
            "roots": ["Cargo.toml", "apps"],
            "operation": { "type": "cfworker" },
            "reuse": reuse,
            "keys": {
                "workload": "1".repeat(64),
                "proof": "2".repeat(64),
                "publication": "3".repeat(64)
            }
        })
        .to_string()
    };
    let first = run(
        root,
        &request(serde_json::json!({"type": "none", "source": ""})),
        None,
    );
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let stdout = String::from_utf8_lossy(&first.stdout);
    let result: serde_json::Value =
        serde_json::from_str(stdout.lines().last().expect("ship result")).expect("ship result");
    assert_eq!(result["result"]["type"], "url");
    let workload = root.join("target/cfworker/probe-worker.tar.gz");
    assert!(workload.is_file());
    let before = calls(root).matches("--filter @probe/web build").count();
    std::fs::remove_dir_all(root.join("apps/web/dist")).expect("clear built tree");
    let held = run(
        root,
        &request(serde_json::json!({
            "type": "workload",
            "source": "https://workflow.example/worker.tgz"
        })),
        Some(&workload),
    );
    assert!(
        held.status.success(),
        "{}",
        String::from_utf8_lossy(&held.stderr)
    );
    assert_eq!(
        calls(root).matches("--filter @probe/web build").count(),
        before,
        "a held worker workload must not rebuild"
    );
    assert!(root.join("apps/web/dist/index.html").is_file());
}

fn run(root: &Path, request: &str, workload: Option<&Path>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(["ship", "execute", "--request", request])
        .current_dir(root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                root.join("bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("SITE_CALLS", root.join("calls"))
        .env("SITE_CASE", "live")
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_CHANNEL", "beta")
        .env("PLUMB_RELEASE_VERSION", "v1.2.3-beta.1")
        .env("PLUMB_SITE_API", "https://cloud.test")
        .env("PLUMB_SITE_TOKEN", "secret")
        .env("PLUMB_SITE_TURNS", "1")
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
        );
    if let Some(workload) = workload {
        command.env("FAKE_WORKER_WORKLOAD", workload);
    }
    command.output().expect("worker request")
}

fn calls(root: &Path) -> String {
    std::fs::read_to_string(root.join("calls")).expect("worker calls")
}
