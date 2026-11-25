use super::operation;
use crate::runtime::observation::{Declaration, Run};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;

#[test]
fn verbs() {
    assert_eq!(operation(&["fetch", "origin"]), Some("git.fetch"));
    assert_eq!(
        operation(&["-C", "/tmp/secret", "push", "origin", "main"]),
        Some("git.push")
    );
    assert_eq!(
        operation(&["--no-pager", "ls-remote", "--tags", "https://tok@host/r"]),
        Some("git.ls-remote")
    );
    assert_eq!(operation(&["rev-parse", "HEAD"]), None);
    assert_eq!(operation(&["status", "--porcelain"]), None);
}

#[cfg(all(feature = "depot", feature = "vendor"))]
#[test]
fn exchanges() {
    enabled();
    disabled();
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn enabled() {
    let home = tempfile::tempdir().unwrap();
    let file = home.path().join("plumb.toml");
    std::fs::write(
        &file,
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
    )
    .unwrap();
    let buffer = home.path().join("state/locus");
    let run = Run::start(
        Declaration {
            producer: "plumb",
            file: &file,
            buffer: buffer.clone(),
        },
        "release",
    )
    .unwrap();
    bucket();
    github();
    git();
    run.finish(0);

    let atoms = records(&buffer);
    let calls: Vec<&Value> = atoms
        .iter()
        .filter(|atom| atom["payload"]["event"] == "provider.call")
        .collect();
    assert_eq!(calls.len(), 5, "{calls:?}");

    assert_eq!(calls[0]["payload"]["provider"], "https");
    assert_eq!(calls[0]["payload"]["operation"], "bucket.fetch");
    assert_eq!(calls[0]["payload"]["outcome"], "ok");
    assert_eq!(calls[0]["payload"]["status"], 200);
    assert_eq!(calls[0]["payload"]["attempts"], 2);

    assert_eq!(calls[1]["payload"]["provider"], "https");
    assert_eq!(calls[1]["payload"]["operation"], "bucket.fetch");
    assert_eq!(calls[1]["payload"]["outcome"], "missing");
    assert_eq!(calls[1]["payload"]["status"], 404);
    assert_eq!(calls[1]["payload"]["attempts"], 1);

    assert_eq!(calls[2]["payload"]["provider"], "github");
    assert_eq!(calls[2]["payload"]["operation"], "rest.get.repository");
    assert_eq!(calls[2]["payload"]["transport"], "rest");
    assert_eq!(calls[2]["payload"]["outcome"], "observed");
    assert_eq!(calls[2]["payload"]["status"], 200);
    assert_eq!(calls[2]["payload"]["attempts"], 1);

    assert_eq!(calls[3]["payload"]["provider"], "git");
    assert_eq!(calls[3]["payload"]["operation"], "git.fetch");
    assert_eq!(calls[3]["payload"]["outcome"], "ok");

    assert_eq!(calls[4]["payload"]["provider"], "git");
    assert_eq!(calls[4]["payload"]["operation"], "git.ls-remote");
    assert_eq!(calls[4]["payload"]["outcome"], "ok");

    for call in &calls {
        assert_eq!(call["producer"], "plumb");
        assert!(call["payload"]["duration_ms"].is_u64());
        assert!(call["context"]["locus.trace"].is_string());
        assert!(call["context"]["locus.span"].is_string());
    }
    let text = serde_json::to_string(&calls).unwrap();
    for forbidden in [
        "secret-bucket-body",
        "secret-query",
        "github_pat_secret",
        "SecretOrg",
        "PrivateRepo",
        "secret-bare-remote",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn disabled() {
    let home = tempfile::tempdir().unwrap();
    let file = home.path().join("plumb.toml");
    std::fs::write(&file, "[locus]\nenabled = false\n").unwrap();
    let buffer = home.path().join("state/locus");
    assert!(
        Run::start(
            Declaration {
                producer: "plumb",
                file: &file,
                buffer: buffer.clone(),
            },
            "release",
        )
        .is_none()
    );
    bucket();
    github();
    git();
    assert!(!buffer.exists());
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn bucket() {
    const REPLIES: &[&[u8]] = &[
        b"",
        b"HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\nsecret-bucket-body",
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    ];
    let (url, server) = serve(REPLIES);
    let found = crate::seat::bucket::fetch(&format!("{url}?token=secret-query"))
        .unwrap()
        .unwrap();
    assert_eq!(found, b"secret-bucket-body");
    assert_eq!(crate::seat::bucket::fetch(&url).unwrap(), None);
    server.join().unwrap();
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn github() {
    let body =
        r#"{"id":14,"full_name":"SecretOrg/PrivateRepo","private":true,"default_branch":"main"}"#;
    let reply = format!(
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        drain(&stream);
        stream.write_all(reply.as_bytes()).unwrap();
    });
    let client = runseal::github::Client::loopback("github_pat_secret", address).unwrap();
    let repo = client.repository("SecretOrg", "PrivateRepo").unwrap().value;
    assert_eq!(repo.name, "SecretOrg/PrivateRepo");
    server.join().unwrap();
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn git() {
    let root = tempfile::tempdir().unwrap();
    let bare = root.path().join("secret-bare-remote.git");
    let work = root.path().join("work");
    std::fs::create_dir_all(&bare).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    run(&bare, &["init", "--bare", "-q"]);
    run(&work, &["init", "-q", "-b", "main"]);
    run(&work, &["config", "user.name", "Test"]);
    run(&work, &["config", "user.email", "test@example.com"]);
    std::fs::write(work.join("note.txt"), "ok\n").unwrap();
    run(&work, &["add", "note.txt"]);
    run(&work, &["commit", "-q", "-m", "seed"]);
    run(
        &work,
        &["remote", "add", "origin", bare.to_str().expect("utf8")],
    );
    run(&work, &["push", "-q", "origin", "main"]);
    let report = crate::land::rejoin::report(&work).unwrap();
    assert_eq!(report.state, "unmarked");
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn serve(replies: &'static [&'static [u8]]) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/distribution.json",
        listener.local_addr().unwrap()
    );
    let server = std::thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            drain(&stream);
            stream.write_all(reply).unwrap();
        }
    });
    (url, server)
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn drain(stream: &std::net::TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap() > 2 {
        line.clear();
    }
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn run(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[cfg(all(feature = "depot", feature = "vendor"))]
fn records(buffer: &Path) -> Vec<Value> {
    let mut atoms = Vec::new();
    for entry in std::fs::read_dir(buffer).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            let content = std::fs::read_to_string(path).unwrap();
            atoms.extend(
                content
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).unwrap()),
            );
        }
    }
    atoms
}
