use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn depot() -> (String, String, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let source = format!("http://127.0.0.1:{port}");
    let version = plumb::version!("PLUMB");
    let manifest = plumb::depot::v3::Manifest::new(
        plumb::depot::v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: version.into(),
            marker: plumb::depot::v3::Marker {
                name: version.into(),
                sha256: "a".repeat(64),
            },
            kind: plumb::depot::v3::Kind::Skill,
        },
        vec![plumb::depot::v3::Object {
            path: "SKILL.md".into(),
            sha256: plumb::depot::sha(b"# fixture\n"),
            size: 10,
            media: "text/plain".into(),
            executable: false,
        }],
    )
    .expect("manifest");
    let pointer = plumb::depot::v3::Pointer::new(
        &manifest,
        plumb::depot::v3::Publication {
            source: &source,
            prior: None,
            created: "2026-09-01T01:02:03Z".into(),
        },
    )
    .expect("pointer");
    let digest = pointer.generation.clone();
    let generation = pointer
        .manifest
        .url
        .strip_suffix(plumb::depot::v3::LEAF)
        .expect("generation URL")
        .to_string();
    let pointer = pointer.encode().expect("pointer body");
    let manifest = manifest.encode().expect("manifest body");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).expect("request");
            let request = String::from_utf8_lossy(&request[..read]);
            let body = if request.contains("latest.json") {
                &pointer
            } else {
                &manifest
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).expect("head");
            stream.write_all(body).expect("body");
        }
    });
    (source, generation, digest)
}

fn managed(root: &Path, version: &str, url: &str, sha: &str) -> (PathBuf, PathBuf) {
    let state = root.join("plumb");
    let skill = root.join("agent/skills/plumb");
    fs::create_dir_all(state.join("state")).expect("state");
    fs::create_dir_all(&skill).expect("skill");
    fs::write(
        skill.join("metadata.json"),
        format!(
            "{{\n  \"schema\": 1,\n  \"name\": \"plumb\",\n  \"version\": \"{version}\",\n  \"keeper\": \"plumb\"\n}}\n"
        ),
    )
    .expect("marker");
    fs::write(skill.join("SKILL.md"), "# fixture\n").expect("brief");
    fs::write(
        state.join("state/skills.json"),
        format!(
            "{{\n  \"schema\": 1,\n  \"records\": [{{\n    \"agent\": \"fixture\",\n    \"path\": {:?},\n    \"version\": \"{version}\",\n    \"url\": \"{url}\",\n    \"sha\": \"{sha}\"\n  }}]\n}}\n",
            skill
        ),
    )
    .expect("ledger");
    (state, skill)
}

fn plumb(home: &Path, depot: &str, arguments: &[&str]) -> Output {
    super::support::stock(&home.join("configurations"), &[]);
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", home)
        .env("PLUMB_RULES_SOURCE", depot)
        .args(arguments)
        .output()
        .expect("plumb")
}

#[test]
fn contract() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (depot, url, sha) = depot();
    let (home, _) = managed(fixture.path(), plumb::version!("PLUMB"), &url, &sha);

    let status = plumb(&home, &depot, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert_eq!(status["operation"], "status");
    assert_eq!(status["target"]["version"], plumb::version!("PLUMB"));
    assert_eq!(status["seats"][0]["state"], "current");
    assert_eq!(status["seats"][0]["action"], "none");

    let dry = plumb(&home, &depot, &["skill", "upgrade", "--dry-run", "--json"]);
    assert!(dry.status.success());
    let dry: serde_json::Value = serde_json::from_slice(&dry.stdout).expect("dry-run json");
    assert_eq!(dry["operation"], "upgrade_dry_run");
    assert_eq!(dry["seats"][0]["action"], "none");

    let current = plumb(&home, &depot, &["skill", "upgrade", "--json"]);
    assert!(current.status.success());
    let current: serde_json::Value = serde_json::from_slice(&current.stdout).expect("upgrade json");
    assert!(current["changed"].as_array().expect("changed").is_empty());
    assert_eq!(current["unchanged"].as_array().expect("unchanged").len(), 1);
}

#[test]
fn ahead() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (depot, url, sha) = depot();
    let (home, _) = managed(fixture.path(), "v2.0.0", &url, &sha);

    let status = plumb(&home, &depot, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert_eq!(status["seats"][0]["state"], "ahead");
    assert_eq!(status["seats"][0]["action"], "refuse");

    let dry = plumb(&home, &depot, &["skill", "upgrade", "--dry-run", "--json"]);
    assert!(!dry.status.success());
    let dry: serde_json::Value = serde_json::from_slice(&dry.stdout).expect("dry-run json");
    assert_eq!(dry["seats"][0]["action"], "refuse");
}

#[test]
fn unmanaged() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (depot, _, _) = depot();
    let home = fixture.path().join("plumb");

    let status = plumb(&home, &depot, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert!(status["seats"].as_array().expect("seats").is_empty());

    let dry = plumb(&home, &depot, &["skill", "upgrade", "--dry-run", "--json"]);
    assert!(!dry.status.success());
}
