use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

#[test]
fn exact() {
    let fixture = tempfile::tempdir().expect("worker fixture");
    let inventory = crate::support::Bucket::open(11);
    let root = fixture.path();
    super::seed(root);
    std::fs::write(
        root.join("plumb.toml"),
        "[release.cfworker]\naccount = \"account\"\ndomain = \"site.test\"\n",
    )
    .expect("release manifest");
    let request = |reuse: serde_json::Value| {
        serde_json::json!({
            "schema": "plumb.ship-request/v2",
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
        &inventory.endpoint(),
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
    assert_eq!(result["depot"]["schema"], "plumb.depot-worker/v1");
    assert_eq!(result["depot"]["marker"], "v1.2.3-beta.1");
    assert_eq!(result["depot"]["worker"], "probe");
    assert_eq!(result["depot"]["version"], "abcdefgh12345678");
    let projected = calls(root);
    assert!(
        projected.contains("wrangler versions upload --tag v1.2.3-beta.1"),
        "{projected}"
    );
    assert!(
        !projected.contains("wrangler versions deploy"),
        "ship must not move worker traffic: {projected}"
    );
    let workload = root.join("target/cfworker/probe-worker.tar.gz");
    assert!(workload.is_file());
    let source = format!(
        "https://workflow.example/workloads/{:x}.tgz",
        Sha256::digest(std::fs::read(&workload).expect("worker workload"))
    );
    let before = calls(root).matches("--filter @probe/web build").count();
    let prepared = calls(root).matches("corepack enable").count();
    std::fs::remove_dir_all(root.join("apps/web/dist")).expect("clear built tree");
    let held = run(
        root,
        &request(serde_json::json!({
            "type": "workload",
            "source": source
        })),
        Some(&workload),
        &inventory.endpoint(),
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
    assert_eq!(
        calls(root).matches("corepack enable").count(),
        prepared + 1,
        "a held worker workload still needs the pnpm executable"
    );
    assert!(root.join("apps/web/dist/index.html").is_file());
    inventory.finish();
}

fn run(
    root: &Path,
    request: &str,
    workload: Option<&Path>,
    inventory: &str,
) -> std::process::Output {
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
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", inventory)
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

#[test]
fn depot() {
    let fixture = tempfile::tempdir().expect("worker fixture");
    let bare = tempfile::tempdir().expect("bare fixture");
    let root = fixture.path();
    super::seed(root);
    std::fs::remove_file(root.join("bin/git")).expect("real Git must resolve the marker");
    std::fs::write(
        root.join("plumb.toml"),
        concat!(
            "[release]\n",
            "product = \"probe\"\n",
            "authority = \"https://releases.test\"\n",
            "[release.cfworker]\n",
            "account = \"account\"\n",
            "domain = \"site.test\"\n",
        ),
    )
    .expect("release manifest");
    std::fs::create_dir_all(root.join(".plumb/releases/v1.2.3")).expect("datum root");
    std::fs::write(
        root.join(".plumb/releases/v1.2.3/datum.toml"),
        "schema = 1\nversion = \"v1.2.3\"\n",
    )
    .expect("datum");
    git(
        None,
        &["init", "-q", "--bare", bare.path().to_str().expect("bare")],
    );
    git(Some(root), &["init", "-q"]);
    git(Some(root), &["config", "user.name", "Plumb Test"]);
    git(
        Some(root),
        &["config", "user.email", "plumb@example.invalid"],
    );
    git(
        Some(root),
        &[
            "remote",
            "add",
            "origin",
            &format!("file://{}", bare.path().display()),
        ],
    );
    git(Some(root), &["add", "."]);
    git(Some(root), &["commit", "-qm", "candidate"]);
    git(
        Some(root),
        &["tag", "-a", "v1.2.3-beta.1", "-m", "probe v1.2.3-beta.1"],
    );
    for reference in [
        "HEAD:refs/heads/main",
        "HEAD:refs/heads/release/v1.2.3",
        "refs/tags/v1.2.3-beta.1",
    ] {
        git(Some(root), &["push", "-q", "origin", reference]);
    }
    let request = serde_json::json!({
        "schema": "plumb.depot-worker/v1",
        "marker": "v1.2.3-beta.1",
        "worker": "probe",
        "version": "abcdefgh12345678",
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "depot",
            "worker",
            "--marker",
            "v1.2.3-beta.1",
            "--request",
            &request,
        ])
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
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_SITE_API", "https://cloud.test")
        .env("PLUMB_SITE_TOKEN", "secret")
        .env("PLUMB_SITE_TURNS", "1")
        .output()
        .expect("depot worker");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let projected = calls(root);
    assert!(
        projected.contains("wrangler versions deploy abcdefgh12345678@100% --name probe -y"),
        "{projected}"
    );
}

fn git(root: Option<&Path>, args: &[&str]) {
    let mut command = Command::new("git");
    if let Some(root) = root {
        command.arg("-C").arg(root);
    }
    let output = command.args(args).output().expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
