use super::{Factory, Grant, Resource, serve};
use serde_json::json;

fn grant() -> Grant {
    Grant {
        name: "depot:marker:invocation".into(),
        permission: "permit".into(),
        resource: Resource::Exact(
            "com.cloudflare.edge.r2.bucket.account_default_perish-probe-releases".into(),
        ),
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
