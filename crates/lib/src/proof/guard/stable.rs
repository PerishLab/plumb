use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;

use super::{Action, Authority, Descriptor, Expected, SCHEMA};

const OLD: &str = "1111111111111111111111111111111111111111";
const NEW: &str = "6666666666666666666666666666666666666666";
const DEPOT: &str = "7777777777777777777777777777777777777777777777777777777777777777";
const EMPTY: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
const SEAL: &str = "/v1/releases/stable/v0.2.0/seal.json";

struct Release {
    guard: Option<String>,
    pinned: Option<String>,
}

fn guard() -> String {
    format!(r#"{{"producer":"v0.2.0@{NEW}","depot":"{DEPOT}"}}"#)
}

fn serve(release: Release) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let mut seal =
        r#"{"schema":1,"channel":"stable","releaseVersion":"v0.2.0","artifacts":{}"#.to_string();
    if let Some(guard) = release.guard {
        seal.push_str(&format!(r#","guard":{guard}"#));
    }
    seal.push('}');
    let sha = release
        .pinned
        .unwrap_or_else(|| crate::skill::stamp(seal.as_bytes()));
    let pointer = format!(
        r#"{{"schema":1,"channel":"stable","releaseVersion":"v0.2.0","seal":{{"name":"seal.json","url":"http://127.0.0.1:{port}{SEAL}","sha256":"{sha}"}}}}"#
    );
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut buffer = [0u8; 1024];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let head = String::from_utf8_lossy(&buffer[..read]);
            let body = match head.split_whitespace().nth(1) {
                Some("/v1/channels/stable.json") => pointer.as_bytes(),
                Some(SEAL) => seal.as_bytes(),
                _ => &[][..],
            };
            let mut out = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            out.extend_from_slice(body);
            let _ = stream.write_all(&out);
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn online() -> String {
    serve(Release {
        guard: Some(guard()),
        pinned: None,
    })
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=probe",
            "-c",
            "user.email=probe@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .status()
        .expect("git");
    assert!(status.success());
}

fn committed(root: &Path) -> Expected {
    let mut proof = Descriptor {
        resolution: None,
        schema: SCHEMA.into(),
        repository: "PerishLab/probe".into(),
        tree: EMPTY.into(),
        plumb: format!("v0.2.0@{NEW}"),
        depot: DEPOT.into(),
        platform: crate::config::platform(),
        actions: vec![Action {
            name: "guard/test".into(),
            input: "4".repeat(64),
            world: "5".repeat(64),
        }],
        digest: String::new(),
    };
    proof.digest = proof.seal().expect("seal");
    let token = proof.encode().expect("encode");
    let message = format!("probe\n\n{}{token}", super::TRAILER);
    git(
        root,
        &[
            "commit",
            "-q",
            "--no-verify",
            "--allow-empty",
            "-m",
            &message,
        ],
    );
    Expected::held(&proof)
}

#[test]
fn reads() {
    let authority = Authority::published(&online()).expect("stable authority");
    assert_eq!(authority.producer(), format!("v0.2.0@{NEW}"));
    assert_eq!(authority.depot(), DEPOT);
}

#[test]
fn mismatch() {
    let error = Authority::published(&serve(Release {
        guard: Some(guard()),
        pinned: Some("8".repeat(64)),
    }))
    .expect_err("seal digest mismatch must refuse");
    assert!(error.contains("digest mismatch"), "{error}");
}

#[test]
fn missing() {
    let error = Authority::published(&serve(Release {
        guard: None,
        pinned: None,
    }))
    .expect_err("seal without guard must refuse");
    assert!(error.contains("carries no guard authority"), "{error}");
}

#[test]
fn foreign() {
    let error = Authority::published(&serve(Release {
        guard: Some(format!(
            r#"{{"producer":"v0.1.0@{NEW}","depot":"{DEPOT}"}}"#
        )),
        pinned: None,
    }))
    .expect_err("producer of another release must refuse");
    assert!(error.contains("names guard producer v0.1.0"), "{error}");
}

#[test]
fn offline() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let base = format!("http://127.0.0.1:{port}");
    let error = Authority::published(&base).expect_err("unreachable authority must refuse");
    assert!(
        error.contains(&format!("{base}/v1/channels/stable.json")),
        "{error}"
    );
}

#[test]
fn newer() {
    let repository = super::verify::seat();
    let root = repository.path();
    let expected = committed(root);
    let compiled = Authority::fixture(format!("v0.1.0@{OLD}"), &"2".repeat(64));
    let stable = Authority::published(&online()).expect("stable authority");
    assert!(
        compiled
            .verify(root, "HEAD", &expected)
            .expect_err("compiled authority must refuse a newer producer")
            .contains("released authority")
    );
    assert!(Authority::among(vec![compiled.clone()], root, "HEAD").is_err());
    let chosen =
        Authority::among(vec![compiled, stable.clone()], root, "HEAD").expect("stable accepted");
    assert_eq!(chosen, stable);
    chosen
        .verify(root, "HEAD", &expected)
        .expect("stable authority verifies the newer proof");
}
