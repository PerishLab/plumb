use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

enum Court {
    Dispatch,
    Prepare(bool),
}

fn repo(root: &Path, origin: &str) {
    std::fs::write(
        root.join("plumb.toml"),
        r#"[release]
product = "probe"
authority = "https://releases.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]
"#,
    )
    .expect("manifest");
    run(Command::new("git").args(["init", "-q"]).current_dir(root));
    run(Command::new("git")
        .args(["remote", "add", "origin", origin])
        .current_dir(root));
}

fn command(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .current_dir(root)
        .env("FORGEJO_TOKEN", "test-token")
        .env("HARNESS_RUN_POLL_MS", "1")
        .env("HARNESS_RUN_TIMEOUT_MS", "100")
        .output()
        .expect("plumb")
}

fn run(command: &mut Command) {
    let output = command.output().expect("command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn serve(court: Court, count: usize) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().take(count).flatten() {
            let (request, body) = request(&mut stream);
            seen.lock().expect("calls").push(request.clone());
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
        Court::Dispatch if request.contains("/dispatches ") => {
            ("201 Created", json!({"id": 88, "run_number": 7}))
        }
        Court::Dispatch if request.contains("/actions/runs/88 ") => {
            ("200 OK", json!({"id": 88, "status": "success"}))
        }
        Court::Prepare(_) if request.contains("GET /api/v1/user ") => {
            ("200 OK", json!({"login": "operator"}))
        }
        Court::Prepare(_) if request.contains("GET ") && request.contains("branch_protections") => {
            ("404 Not Found", json!({"message": "missing"}))
        }
        Court::Prepare(exact)
            if request.contains("POST ") && request.contains("branch_protections") =>
        {
            let mut value = body;
            value["branch_name"] = json!("release/v1.2.0");
            if !exact {
                value["enable_push"] = json!(false);
            }
            ("201 Created", value)
        }
        Court::Prepare(_) if request.contains("GET ") && request.contains("/branches/") => {
            ("404 Not Found", json!({"message": "missing"}))
        }
        Court::Prepare(_)
            if request.contains("POST ") && request.ends_with("/branches HTTP/1.1") =>
        {
            ("201 Created", json!({"name": "release/v1.2.0"}))
        }
        _ => ("500 Internal Server Error", json!({"message": request})),
    }
}

#[test]
fn dispatch() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, calls) = serve(Court::Dispatch, 2);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "release",
            "dispatch",
            "--channel",
            "nightly",
            "--version",
            "v1.2.0-nightly.1",
            "--watch",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("triggered release-exact.yml run 88"),
        "{text}"
    );
    assert!(text.contains("run 88: success"), "{text}");
    let calls = calls.lock().expect("calls");
    assert!(calls.iter().any(|call| call.contains("/dispatches ")));
    assert!(calls.iter().any(|call| call.contains("/actions/runs/88 ")));
}

#[test]
fn protection() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Prepare(true), 5);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(fixture.path(), &["stable", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("prepared release/v1.2.0 from main"));
}

#[test]
fn drift() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Prepare(false), 3);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(fixture.path(), &["stable", "prepare", "--version", "1.2.0"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("branch protection did not read back byte-for-byte")
    );
}

#[test]
fn freedom() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(
        fixture.path(),
        "ssh://git@git.perish.top/PerishLab/probe.git",
    );
    let output = command(
        fixture.path(),
        &[
            "release",
            "dispatch",
            "--channel",
            "nightly",
            "--version",
            "v1.2.0-nightly.9",
            "--ref",
            "topic/candidate",
            "--dry-run",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ref=topic/candidate"), "{text}");
    assert!(text.contains(r#""channel":"nightly""#), "{text}");
}

#[test]
fn strict() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(
        fixture.path(),
        "ssh://git@git.perish.top/PerishLab/probe.git",
    );
    let output = command(
        fixture.path(),
        &[
            "release",
            "dispatch",
            "--channel",
            "stable",
            "--version",
            "v1.2.0",
            "--promotion-version",
            "v1.2.0-beta.4",
            "--ref",
            "main",
            "--dry-run",
        ],
    );
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("stable ref is derived"), "{text}");
}
