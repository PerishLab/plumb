#[path = "../../src/command/release/authority/cloudflare.rs"]
mod adapter;

use adapter::{Bucket, Custom, Factory, Grant};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn lifecycle() {
    let _: Option<Custom> = None;
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
            resource: "resource".into(),
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

fn serve(answers: Vec<&str>) -> (String, thread::JoinHandle<Vec<String>>) {
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
                request.lines().next().unwrap_or("").to_string()
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
