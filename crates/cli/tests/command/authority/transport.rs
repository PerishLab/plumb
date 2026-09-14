use super::super::{cloud, depot::support};
use serde_json::{Value, json};
use std::process::Command;

fn policy(full: bool) -> Value {
    let resources = if full {
        json!({"com.cloudflare.edge.r2.bucket.account_default_perish-concord-releases":"*"})
    } else {
        json!({})
    };
    json!({"id":"writer", "name":"ship:release-buckets", "status":"active",
        "policies":[{"effect":"allow", "permission_groups":[{"id":"permit"}],
            "resources": resources}]})
}

fn answer(value: Value) -> String {
    json!({"success":true,"result":value}).to_string()
}

fn observe(full: bool) -> Vec<String> {
    vec![
        answer(json!({"status":"active"})),
        answer(json!([{"id":"writer","name":"ship:release-buckets"}])),
        answer(policy(full)),
        answer(json!([{"id":"permit"}])),
        json!([
            {"name":"SHIP_PUBLISH_S3_ACCESS_KEY"},
            {"name":"SHIP_PUBLISH_S3_SECRET_KEY"},
            {"name":"SHIP_PUBLISH_S3_ENDPOINT"},
            {"name":"SHIP_PUBLISH_FINGERPRINT"}
        ])
        .to_string(),
    ]
}

fn command(root: &std::path::Path, home: &std::path::Path, url: &str) -> Command {
    for args in [
        vec!["init", "-q"],
        vec![
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishLab/plumb.git",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
    let file = root.join("token");
    std::fs::write(&file, "fixture-secret\n").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(["authority", "ship", "--json"])
        .arg(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_AUTHORITY_ACCOUNT", "account")
        .env("PLUMB_AUTHORITY_API", url)
        .env("PLUMB_AUTHORITY_TOKEN", "")
        .env("PLUMB_AUTHORITY_TOKEN_FILE", file)
        .env("FORGEJO_URL", url.trim_end_matches("/client/v4"))
        .env("FORGEJO_TOKEN", "fixture-forgejo")
        .env_remove("FORGEJO_TOKEN_FILE");
    command
}

fn home() -> tempfile::TempDir {
    support::depot(&[(
        "rules/products.toml",
        "schema='plumb.products/v1'\n[[product]]\nidentity='git.perish.top/PerishFire/concord'\nname='concord'\nauthority='https://releases.concord.perish.uk'\nderivatives=['skill']\n",
    )])
}

#[test]
fn unbound() {
    let root = tempfile::tempdir().unwrap();
    let home = home();
    let mut answers = observe(false);
    answers.extend(observe(false));
    for value in [
        policy(false),
        json!([{"id":"permit"}]),
        policy(false),
        json!([{"id":"permit"}]),
        policy(true),
        policy(true),
        json!([{"id":"permit"}]),
    ] {
        answers.push(answer(value));
    }
    answers.extend(observe(true));
    let (url, server) = cloud::serve(answers.iter().map(String::as_str).collect());
    let output = command(root.path(), home.path(), &url)
        .arg("--apply")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["state"], "ready", "{report}");
    assert!(
        report["note"]
            .as_str()
            .unwrap()
            .contains("escrow is absent")
    );
    let calls = server.join().unwrap();
    assert_eq!(
        calls.iter().filter(|call| call.starts_with("PUT ")).count(),
        1
    );
    assert!(
        calls
            .iter()
            .filter(|call| !call.starts_with("GET "))
            .all(|call| call.starts_with("PUT /client/v4/accounts/account/tokens/writer "))
    );
    assert!(!root.path().join(".local/ship-authority.env").exists());
}

#[test]
fn direct() {
    let root = tempfile::tempdir().unwrap();
    let home = home();
    let answers = observe(true);
    let (url, server) = cloud::serve(answers.iter().map(String::as_str).collect());
    let output = command(root.path(), home.path(), &url)
        .env("PLUMB_AUTHORITY_TOKEN", "other-secret")
        .env("PLUMB_AUTHORITY_TOKEN_FILE", root.path().join("missing"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!error.contains("other-secret"));
    assert!(
        server
            .join()
            .unwrap()
            .iter()
            .all(|call| call.starts_with("GET "))
    );
}
