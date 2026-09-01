use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn releases() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || {
        let seal = format!(
            r#"{{"schema":1,"channel":"stable","releaseVersion":"v1.2.3","artifacts":{{"skill":{{"name":"plumb-skill.tar.gz","url":"http://127.0.0.1:{port}/plumb-skill.tar.gz","sha256":"abc"}}}}}}"#
        );
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).expect("request");
            let request = String::from_utf8_lossy(&request[..read]);
            let body = if request.contains("/v1/channels/stable.json") {
                format!(
                    r#"{{"schema":1,"channel":"stable","releaseVersion":"v1.2.3","seal":{{"name":"seal.json","url":"http://127.0.0.1:{port}/v1/releases/stable/v1.2.3/seal.json","sha256":"{}"}}}}"#,
                    plumb::skill::stamp(seal.as_bytes())
                )
            } else {
                seal.clone()
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).expect("head");
            stream.write_all(body.as_bytes()).expect("body");
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn managed(root: &Path, version: &str) -> (PathBuf, PathBuf) {
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
            "{{\n  \"schema\": 1,\n  \"records\": [{{\n    \"agent\": \"fixture\",\n    \"path\": {:?},\n    \"version\": \"{version}\",\n    \"url\": \"https://releases.example.test/plumb-skill.tar.gz\",\n    \"sha\": \"abc\"\n  }}]\n}}\n",
            skill
        ),
    )
    .expect("ledger");
    (state, skill)
}

fn plumb(home: &Path, releases: &str, arguments: &[&str]) -> Output {
    super::support::stock(&home.join("configurations"), &[]);
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", home)
        .env("PLUMB_RELEASES", releases)
        .args(arguments)
        .output()
        .expect("plumb")
}

#[test]
fn contract() {
    let fixture = tempfile::tempdir().expect("fixture");
    let releases = releases();
    let (home, _) = managed(fixture.path(), "v1.2.3");

    let status = plumb(&home, &releases, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert_eq!(status["operation"], "status");
    assert_eq!(status["target"]["version"], "v1.2.3");
    assert_eq!(status["seats"][0]["state"], "current");
    assert_eq!(status["seats"][0]["action"], "none");

    let dry = plumb(
        &home,
        &releases,
        &["skill", "upgrade", "--dry-run", "--json"],
    );
    assert!(dry.status.success());
    let dry: serde_json::Value = serde_json::from_slice(&dry.stdout).expect("dry-run json");
    assert_eq!(dry["operation"], "upgrade_dry_run");
    assert_eq!(dry["seats"][0]["action"], "none");

    let current = plumb(&home, &releases, &["skill", "upgrade", "--json"]);
    assert!(current.status.success());
    let current: serde_json::Value = serde_json::from_slice(&current.stdout).expect("upgrade json");
    assert!(current["changed"].as_array().expect("changed").is_empty());
    assert_eq!(current["unchanged"].as_array().expect("unchanged").len(), 1);
}

#[test]
fn ahead() {
    let fixture = tempfile::tempdir().expect("fixture");
    let releases = releases();
    let (home, _) = managed(fixture.path(), "v2.0.0");

    let status = plumb(&home, &releases, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert_eq!(status["seats"][0]["state"], "ahead");
    assert_eq!(status["seats"][0]["action"], "refuse");

    let dry = plumb(
        &home,
        &releases,
        &["skill", "upgrade", "--dry-run", "--json"],
    );
    assert!(!dry.status.success());
    let dry: serde_json::Value = serde_json::from_slice(&dry.stdout).expect("dry-run json");
    assert_eq!(dry["seats"][0]["action"], "refuse");
}

#[test]
fn unmanaged() {
    let fixture = tempfile::tempdir().expect("fixture");
    let releases = releases();
    let home = fixture.path().join("plumb");

    let status = plumb(&home, &releases, &["skill", "status", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status json");
    assert!(status["seats"].as_array().expect("seats").is_empty());

    let dry = plumb(
        &home,
        &releases,
        &["skill", "upgrade", "--dry-run", "--json"],
    );
    assert!(!dry.status.success());
}
