use super::{Factory, Grant, Resource, serve};
use serde_json::{Value, json};

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

fn held(grant: &Grant) -> Value {
    json!({
        "id":"temporary", "name":grant.name, "status":"active",
        "expires_on":grant.expires,
        "policies":[{
            "effect":"allow", "resources":grant.resource.policy(),
            "permission_groups":[{"id":grant.permission}]
        }]
    })
}

fn answers(held: Value, revoke: bool) -> Vec<String> {
    vec![
        json!({"success":true,"result":{"id":"temporary","value":"private"}}).to_string(),
        json!({"success":true,"result":held}).to_string(),
        json!({"success":revoke,"result":{},"errors":[]}).to_string(),
    ]
}

#[test]
fn closed() {
    for success in [true, false] {
        let grant = grant();
        let answers = answers(held(&grant), true);
        let (url, server) = serve(answers.iter().map(String::as_str).collect());
        let factory = Factory::new("account".into(), url, "factory".into());
        let result = factory.session(&grant, |token| {
            assert_eq!(token.id, "temporary");
            assert_eq!(token.value(), "private");
            if success {
                Ok("projected")
            } else {
                Err("projection failed".into())
            }
        });
        assert_eq!(result.is_ok(), success);
        if !success {
            assert!(result.unwrap_err().contains("projection failed"));
        }
        let calls = server.join().unwrap();
        assert!(calls[0].contains("perish-probe-releases"));
        assert!(calls[0].contains("expires_on"));
        assert!(calls[2].starts_with("DELETE /client/v4/accounts/account/tokens/temporary "));
        assert!(calls.iter().all(|call| !call.contains("private")));
    }
}

#[test]
fn drifted() {
    let grant = grant();
    let mut wider = held(&grant);
    wider["policies"][0]["resources"] = json!({"com.cloudflare.api.account.account":"*"});
    let answers = answers(wider, true);
    let (url, server) = serve(answers.iter().map(String::as_str).collect());
    let factory = Factory::new("account".into(), url, "factory".into());
    let result: Result<(), String> = factory.session(&grant, |_| panic!("scope must verify"));
    assert!(result.unwrap_err().contains("scope"));
    assert!(server.join().unwrap()[2].starts_with("DELETE "));
}

#[test]
fn cleanup() {
    for success in [true, false] {
        let grant = grant();
        let answers = answers(held(&grant), false);
        let (url, server) = serve(answers.iter().map(String::as_str).collect());
        let factory = Factory::new("account".into(), url, "factory".into());
        let result = factory.session(&grant, |_| {
            if success {
                Ok(())
            } else {
                Err("projection failed".into())
            }
        });
        let error = result.unwrap_err();
        assert!(error.contains("could not be revoked"), "{error}");
        assert!(error.contains(if success {
            "completed"
        } else {
            "projection failed"
        }));
        assert!(server.join().unwrap()[2].starts_with("DELETE "));
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
