use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub enum Court {
    Prepare(bool, PathBuf),
    Resume(PathBuf),
    Freeze(PathBuf),
    Rejoin(PathBuf),
}

pub fn serve(court: Court, count: usize) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().take(count).flatten() {
            let (request, body) = request(&mut stream);
            seen.lock()
                .expect("calls")
                .push(format!("{request} {body}"));
            let (status, value) = answer(&court, &request, body);
            let text = value.to_string();
            write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
                text.len()
            )
            .expect("response");
        }
    });
    (format!("http://{address}"), calls)
}

fn request(stream: &mut impl Read) -> (String, Value) {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let end = loop {
        let count = stream.read(&mut buffer).expect("request");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|held| held == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let length = head
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while bytes.len() < end + length {
        let count = stream.read(&mut buffer).expect("body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    let request = head.lines().next().unwrap_or("").to_string();
    let body = serde_json::from_slice(&bytes[end..end + length]).unwrap_or(Value::Null);
    (request, body)
}

fn answer(court: &Court, request: &str, body: Value) -> (&'static str, Value) {
    match court {
        Court::Prepare(_, root) if request.contains("GET /v1/channels/stable.json ") => {
            release(root, "pointer.json")
        }
        Court::Prepare(_, root) if request.contains("GET /v1/releases/stable/") => {
            release(root, "seal.json")
        }
        Court::Prepare(..) | Court::Resume(_) | Court::Rejoin(_)
            if request.contains("GET /api/v1/user ") =>
        {
            ("200 OK", json!({"login": "operator"}))
        }
        Court::Freeze(head) if request.contains("GET ") && request.contains("/branches/") => {
            ("200 OK", cut(head))
        }
        Court::Prepare(..) | Court::Rejoin(_)
            if request.contains("GET ") && request.contains("branch_protections") =>
        {
            (
                "404 Not Found",
                json!({"message": "The target couldn't be found."}),
            )
        }
        Court::Resume(_) if request.contains("GET ") && request.contains("branch_protections") => {
            ("200 OK", json!({"branch_name": "release/v1.2.0"}))
        }
        Court::Resume(_)
            if request.contains("PATCH ") && request.contains("branch_protections") =>
        {
            let mut value = body;
            value["branch_name"] = json!("release/v1.2.0");
            ("200 OK", value)
        }
        Court::Prepare(exact, _)
            if request.contains("POST ") && request.contains("branch_protections") =>
        {
            let mut value = body;
            value["branch_name"] = value["rule_name"].clone();
            if !exact {
                value["enable_push"] = json!(false);
            }
            ("201 Created", value)
        }
        Court::Rejoin(_) if request.contains("POST ") && request.contains("branch_protections") => {
            let mut value = body;
            value["branch_name"] = json!("release/v1.2.0");
            ("201 Created", value)
        }
        Court::Rejoin(_) if request.contains("GET ") && request.contains("/branches/") => (
            "200 OK",
            json!({"commit":{"id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}),
        ),
        Court::Rejoin(settled) if request.contains("GET ") && request.contains("/pulls?") => {
            ("200 OK", pulls(settled))
        }
        Court::Rejoin(_) if request.contains("POST ") && request.ends_with("/pulls HTTP/1.1") => {
            ("201 Created", json!({"number": 12}))
        }
        Court::Rejoin(settled) if request.contains("POST ") && request.contains("/statuses/") => {
            attest(settled, body)
        }
        Court::Rejoin(settled)
            if request.contains("POST ") && request.contains("/pulls/12/merge ") =>
        {
            merge(settled)
        }
        Court::Rejoin(_) if request.contains("GET ") && request.contains("/commits/") => (
            "200 OK",
            json!({"state":"success","statuses":[{
                "context":"guard / guard (pull_request)",
                "status":"success",
                "updated_at":"2026-01-01T00:00:00Z"
            }]}),
        ),
        Court::Prepare(..) if request.contains("GET ") && request.contains("/branches/") => (
            "404 Not Found",
            json!({"message": "The target couldn't be found."}),
        ),
        Court::Resume(head) if request.contains("GET ") && request.contains("/branches/") => {
            ("200 OK", cut(head))
        }
        Court::Prepare(_, head)
            if request.contains("POST ") && request.ends_with("/branches HTTP/1.1") =>
        {
            ("201 Created", cut(head))
        }
        _ => ("500 Internal Server Error", json!({"message": request})),
    }
}

fn attest(settled: &Path, body: Value) -> (&'static str, Value) {
    if body["state"] != "success" || body["context"] != "guard / guard (pull_request)" {
        return (
            "422 Unprocessable Entity",
            json!({"message":"invalid Guard status"}),
        );
    }
    std::fs::write(settled.with_file_name("attested"), "Guard proof").expect("attested");
    ("201 Created", body)
}

fn merge(settled: &Path) -> (&'static str, Value) {
    if !settled.with_file_name("attested").is_file() {
        return (
            "403 Forbidden",
            json!({"message":"required Guard status is missing"}),
        );
    }
    std::fs::write(settled, "settled").expect("settled marker");
    ("204 No Content", Value::Null)
}

fn release(root: &Path, name: &str) -> (&'static str, Value) {
    let path = root.with_file_name(name);
    if !path.is_file() {
        return (
            "404 Not Found",
            json!({"message": "The target couldn't be found."}),
        );
    }
    let bytes = std::fs::read(path).expect("release record");
    (
        "200 OK",
        serde_json::from_slice(&bytes).expect("release JSON"),
    )
}

fn pulls(settled: &Path) -> Value {
    if settled.with_file_name("resume").is_file() {
        json!([{
            "number": 12,
            "state": "open",
            "head": {"ref": "rejoin/v1.2.0"},
            "base": {"ref": "main"}
        }])
    } else {
        json!([{
            "number": 9,
            "state": "closed",
            "head": {"ref": "release/v1.2.0"},
            "base": {"ref": "main"}
        }])
    }
}

fn cut(head: &PathBuf) -> Value {
    let commit = std::fs::read_to_string(head).unwrap_or_default();
    json!({"name": "release/v1.2.0", "commit": {"id": commit.trim()}})
}
