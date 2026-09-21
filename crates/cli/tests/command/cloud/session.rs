use super::{Factory, Grant, Resource, serve};
use serde_json::json;

fn grant() -> Grant {
    Grant {
        name: "depot:marker:invocation".into(),
        permission: "permit".into(),
        resource: Resource::Set {
            account: "account".into(),
            buckets: vec!["perish-probe-releases".into()],
        },
        expires: "2026-09-14T15:00:00Z".into(),
    }
}

#[test]
fn missing() {
    let answers = [
        json!({"success":true,"result":{"id":"temporary"}}).to_string(),
        json!({"success":true,"result":{}}).to_string(),
    ];
    let (url, server) = serve(answers.iter().map(String::as_str).collect());
    let factory = Factory::new("account".into(), url, "factory".into());
    let result = factory.create(&grant());
    assert!(result.err().unwrap().contains("token revoked"));
    assert!(server.join().unwrap()[1].starts_with("DELETE "));
}

#[test]
fn cascade() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("token");
    std::fs::write(&file, "private\n").unwrap();
    for direct in [true, false] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cloud::session::resolved", "--ignored"])
            .env("PLUMB_AUTHORITY_ACCOUNT", "account")
            .env("PLUMB_AUTHORITY_API", "https://factory.test/client/v4/")
            .env("PLUMB_AUTHORITY_TOKEN", if direct { "private" } else { "" })
            .env(
                "PLUMB_AUTHORITY_TOKEN_FILE",
                if direct {
                    root.path().join("missing")
                } else {
                    file.clone()
                },
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
#[ignore = "invoked by cascade in an isolated environment"]
fn resolved() {
    assert_eq!(Factory::resolve().unwrap().id(), "account");
}
