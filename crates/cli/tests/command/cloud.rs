#[path = "../../src/command/release/cloudflare/mod.rs"]
pub(super) mod adapter;

use adapter::{Bucket, Custom, Factory, Grant, Resource};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn lifecycle() {
    let _: Option<Custom> = None;
    assert_eq!(
        Resource::Exact("one-bucket".into()).policy(),
        serde_json::json!({"one-bucket": "*"})
    );
    assert_eq!(
        Resource::Set {
            account: "account".into(),
            buckets: vec!["one".into(), "two".into()],
        }
        .policy(),
        serde_json::json!({
            "com.cloudflare.edge.r2.bucket.account_default_one": "*",
            "com.cloudflare.edge.r2.bucket.account_default_two": "*"
        })
    );
    let answers = vec![
        r#"{"success":true,"result":{"status":"active"}}"#,
        r#"{"success":true,"result":[{"id":"old","name":"stale"}],"result_info":{"total_pages":1}}"#,
        r#"{"success":true,"result":[{"id":"permit","name":"write"}]}"#,
        r#"{"success":true,"result":{"id":"fresh","value":"value-from-api"}}"#,
        r#"{"success":true,"result":{"name":"bucket"}}"#,
        r#"{"success":true,"result":{"name":"bucket"}}"#,
        r#"{"success":true,"result":{"domains":[{"domain":"site.test","zoneId":"zone","enabled":true,"minTLS":"1.2","status":{"ownership":"active","ssl":"active"}}]}}"#,
        r#"{"success":true,"result":{"domains":[{"domain":"site.test","zoneId":"zone","enabled":true,"minTLS":"1.2","status":{"ownership":"active","ssl":"active"}}]}}"#,
        r#"{"success":true,"result":{"domains":[{"domain":"site.test","zoneId":"zone","enabled":true,"minTLS":"1.2","status":{"ownership":"active","ssl":"active"}}]}}"#,
        r#"{"success":true,"result":{}}"#,
        r#"{"success":true,"result":{}}"#,
        r#"{"success":true,"result":{}}"#,
    ];
    let (url, handle) = serve(answers);
    let factory = Factory::new("account".into(), url, "factory-value".into());
    factory.verify().expect("verify");
    let held = factory.held().expect("held");
    assert_eq!(held[0].id, "old");
    assert_eq!(held[0].name, "stale");
    assert_eq!(factory.id(), "account");
    assert_eq!(
        factory.permission("write", "scope").expect("permission"),
        "permit"
    );
    let minted = factory
        .create(&Grant {
            name: "temporary".into(),
            permission: "permit".into(),
            resource: Resource::Set {
                account: "account".into(),
                buckets: vec!["bucket".into()],
            },
            expires: "soon".into(),
        })
        .expect("create");
    assert_eq!(minted.value(), "value-from-api");
    let bucket = Bucket::new(&factory, &minted, "bucket");
    assert!(bucket.live().expect("live"));
    bucket.create().expect("create bucket");
    let domains = bucket.custom().expect("domains");
    assert_eq!(domains[0].domain, "site.test");
    assert_eq!(domains[0].zone, "zone");
    assert!(domains[0].ready());
    assert_eq!(
        domains[0].state(),
        "ownership=active, certificate=active, enabled=true, minimum-tls=1.2"
    );
    assert!(bucket.find("site.test").expect("find domain").is_some());
    bucket.bind("site.test", "zone").expect("bind domain");
    bucket.detach("site.test").expect("detach");
    bucket.erase().expect("erase");
    factory.revoke(&minted.id).expect("revoke");
    let seen = handle.join().expect("server");
    assert_eq!(seen.len(), 12);
    assert!(seen[2].contains("name=write&scope=scope"), "{:?}", seen);
    assert!(seen[6].contains("/r2/buckets/bucket/domains/custom"));
    assert!(seen.iter().all(|line| !line.contains("factory-value")));
    assert!(seen.iter().all(|line| !line.contains("value-from-api")));
}

pub(super) fn serve(answers: Vec<&str>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let answers = answers.into_iter().map(str::to_string).collect::<Vec<_>>();
    let handle = thread::spawn(move || {
        answers
            .into_iter()
            .map(|body| {
                let (mut stream, _) = listener.accept().expect("accept");
                let request = request(&mut stream);
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(reply.as_bytes()).expect("reply");
                format!("{}\n{}", request.lines().next().unwrap_or(""), request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or(""))
            })
            .collect()
    });
    (format!("http://{addr}/client/v4"), handle)
}

fn request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut part = [0u8; 1024];
    loop {
        let count = stream.read(&mut part).expect("read");
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&part[..count]);
        let Some(head) = bytes.windows(4).position(|held| held == b"\r\n\r\n") else {
            continue;
        };
        let length = String::from_utf8_lossy(&bytes[..head])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then_some(value.trim())
            })
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if bytes.len() >= head + 4 + length {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn writer(resources: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": "writer", "name": "ship:release-buckets", "status": "active",
        "expires_on": "2030-01-01T00:00:00Z",
        "condition": {"request.ip": {"in": ["192.0.2.0/24"]}},
        "policies": [{"effect": "allow", "resources": resources,
            "permission_groups": [{"id": "permit"}]}]
    })
}

#[test]
fn policy() {
    let resources = Resource::Set {
        account: "account".into(),
        buckets: vec!["one".into(), "two".into()],
    }
    .policy();
    let before =
        writer(serde_json::json!({"com.cloudflare.edge.r2.bucket.account_default_one": "*"}));
    let after = writer(resources.clone());
    let permit = serde_json::json!([{"id":"permit"}]);
    let responses = [
        before.clone(),
        permit.clone(),
        before.clone(),
        permit.clone(),
        after.clone(),
        after.clone(),
        permit.clone(),
        after,
        permit,
    ]
    .into_iter()
    .map(|value| serde_json::json!({"success":true,"result":value}).to_string())
    .collect::<Vec<_>>();
    let (url, handle) = serve(responses.iter().map(String::as_str).collect());
    let factory = Factory::new("account".into(), url, "factory-value".into());
    factory
        .reconcile("writer", &["one".into(), "two".into()])
        .expect("update policy");
    factory
        .reconcile("writer", &["one".into(), "two".into()])
        .expect("idempotent policy");
    let seen = handle.join().expect("server");
    assert_eq!(
        seen.iter()
            .filter(|request| request.starts_with("PUT "))
            .count(),
        1
    );
    assert!(seen[4].starts_with("PUT /client/v4/accounts/account/tokens/writer "));
    let body: serde_json::Value =
        serde_json::from_str(seen[4].split_once('\n').unwrap().1).unwrap();
    assert_eq!(body["policies"][0]["resources"], resources);
    assert_eq!(body["condition"], before["condition"]);
    assert_eq!(body["expires_on"], before["expires_on"]);
    assert!(
        !seen
            .iter()
            .any(|request| request.starts_with("POST ") || request.starts_with("DELETE "))
    );
}

#[test]
fn stale() {
    let before = writer(serde_json::json!({"one":"*"}));
    let after = writer(serde_json::json!({"two":"*"}));
    let permit = serde_json::json!([{"id":"permit"}]);
    let responses = [before, permit.clone(), after, permit]
        .into_iter()
        .map(|value| serde_json::json!({"success":true,"result":value}).to_string())
        .collect::<Vec<_>>();
    let (url, handle) = serve(responses.iter().map(String::as_str).collect());
    let factory = Factory::new("account".into(), url, "factory-value".into());
    assert!(
        factory
            .reconcile("writer", &["three".into()])
            .unwrap_err()
            .contains("changed before update")
    );
    assert!(
        handle
            .join()
            .unwrap()
            .iter()
            .all(|request| request.starts_with("GET "))
    );
}

#[test]
fn refusal() {
    let resources = serde_json::json!({"one":"*"});
    let before = writer(resources.clone());
    assert!(
        !adapter::Policy::read(before.clone(), resources.clone(), "permit")
            .unwrap()
            .pending()
    );
    assert!(adapter::Policy::read(before.clone(), resources.clone(), "other").is_err());
    let mut inactive = before.clone();
    inactive["status"] = serde_json::json!("disabled");
    assert!(adapter::Policy::read(inactive, resources.clone(), "permit").is_err());
    let mut denied = before;
    denied["policies"][0]["effect"] = serde_json::json!("deny");
    assert!(adapter::Policy::read(denied, resources, "permit").is_err());
}
