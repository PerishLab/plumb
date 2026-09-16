use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

#[test]
fn exact() {
    let fixture = tempfile::tempdir().expect("worker fixture");
    let input = tempfile::tempdir().expect("materialized worker source");
    let root = fixture.path();
    super::seed(root);
    super::seed(input.path());
    super::file(
        &root.join("bin/git"),
        "#!/bin/sh\nexec /usr/bin/git \"$@\"\n",
    );
    let manifest = "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.cfworker]\naccount='account'\ndomain='site.test'\n";
    let binding = crate::marker::prepare(root, &root.join("home"), manifest, "v1.2.3-beta.1");
    let request = |operation: serde_json::Value,
                   source: serde_json::Value,
                   reuse: serde_json::Value| {
        serde_json::json!({
            "schema": "plumb.ship-request/v3", "marker": "v1.2.3-beta.1",
            "configuration": binding["configuration"], "profile": binding["profile"],
            "action": if operation["type"] == "package" { "ship/produce.cfworker" } else { "ship/cfworker" },
            "operation": operation, "input": source, "reuse": reuse,
        }).to_string()
    };
    let production = run(
        root,
        &request(
            serde_json::json!({"type": "package", "operation": {"type": "cfworker"}}),
            serde_json::json!(input.path()),
            serde_json::json!({"type": "none", "source": ""}),
        ),
        None,
    );
    let produced = result(&production);
    let workload = std::path::PathBuf::from(produced["projection"]["workload"].as_str().unwrap());
    assert!(workload.is_file());
    let before = calls(root).matches("--filter @probe/web build").count();
    assert_eq!(before, 1);
    assert!(!calls(root).contains("wrangler versions upload"));
    let source = format!(
        "https://workflow.example/v2/blobs/sha256/{:x}",
        Sha256::digest(std::fs::read(&workload).unwrap())
    );
    std::fs::remove_dir_all(root.join("apps/web/dist")).expect("remove only fixture build output");
    let publication = run(
        root,
        &request(
            serde_json::json!({"type": "cfworker"}),
            serde_json::Value::Null,
            serde_json::json!({"type": "workload", "source": source}),
        ),
        Some(&workload),
    );
    let published = result(&publication);
    assert_eq!(
        published["projection"]["depot"]["schema"],
        "plumb.depot-worker/v1"
    );
    assert_eq!(published["projection"]["depot"]["marker"], "v1.2.3-beta.1");
    assert_eq!(published["projection"]["depot"]["worker"], "probe");
    assert_eq!(
        published["projection"]["depot"]["version"],
        "abcdefgh12345678"
    );
    assert_eq!(published["evidence"]["schema"], "plumb.ship-resource/v1");
    let observed = calls(root);
    assert!(observed.contains("wrangler versions upload --tag v1.2.3-beta.1"));
    assert!(!observed.contains("wrangler versions deploy"));
    assert_eq!(
        observed.matches("--filter @probe/web build").count(),
        before
    );
    assert!(!observed.contains("corepack enable"));
    assert!(root.join("apps/web/dist/index.html").is_file());
}

fn result(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.lines().last().unwrap()).unwrap()
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
        .env("PLUMB_HOME", root.join("home"))
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .env("PLUMB_RELEASE_CHANNEL", "beta")
        .env("PLUMB_RELEASE_VERSION", "v1.2.3-beta.1")
        .env("PLUMB_SITE_API", "https://cloud.test")
        .env("PLUMB_SITE_TOKEN", "secret")
        .env("PLUMB_SITE_TURNS", "1");
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
