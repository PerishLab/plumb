use super::super::world::Fixture;
use super::super::world::run;
use serde_json::{Value, json};
use std::process::Command;

pub(super) fn settlement(command: &impl Fn() -> Command, graph: &Value) {
    let root = command().get_current_dir().unwrap().to_path_buf();
    let git = |args: &[&str]| {
        String::from_utf8(run(Command::new("git").arg("-C").arg(&root).args(args)).stdout)
            .unwrap()
            .trim()
            .to_string()
    };
    let commit = git(&["rev-parse", "v1.2.0^{commit}"]);
    let parent = git(&["rev-parse", "v1.2.0^{}^1"]);
    let path = root.join("releases/v1/releases/stable/v1.2.0/ship.json");
    let rejoin = || {
        command()
            .args(["version", "rejoin", "--version", "v1.2.0", "--dry-run"])
            .env_remove("FORGEJO_TOKEN")
            .env_remove("FORGEJO_TOKEN_FILE")
            .output()
            .unwrap()
    };
    let missing = rejoin();
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("complete stable Ship graph"),
        "{missing:?}"
    );
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, super::held::completion(graph).to_string()).unwrap();
    git(&["update-ref", "refs/remotes/origin/main", &commit]);
    let manifest = root.join("plumb.toml");
    let original = std::fs::read(&manifest).unwrap();
    std::fs::write(&manifest, "not valid TOML [[").unwrap();
    for _ in 0..2 {
        accepted(command, &commit);
        assert_eq!(std::fs::read(&manifest).unwrap(), b"not valid TOML [[");
        assert_eq!(git(&["rev-parse", "HEAD"]), commit);
    }
    git(&["update-ref", "refs/remotes/origin/main", &parent]);
    for action in ["prepare", "freeze"] {
        let refused = command()
            .args(["version", action, "--version", "v1.3.0", "--dry-run"])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("which origin/main does not hold"),
            "{refused:?}"
        );
    }
    git(&["update-ref", "refs/remotes/origin/main", &commit]);
    for status in ["503", "000"] {
        let refused = command()
            .args(["version", "rejoin", "--version", "v1.2.0", "--dry-run"])
            .env("FAKE_COMPLETION_STATUS", status)
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("cannot probe"),
            "{refused:?}"
        );
    }
    std::fs::write(manifest, original).unwrap();
}

fn accepted(command: &impl Fn() -> Command, commit: &str) {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = json!({"commit":{"id":commit}}).to_string();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 4096];
        let read = stream.read(&mut buffer).unwrap();
        assert!(String::from_utf8_lossy(&buffer[..read]).contains("/branches/release"));
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let output = command()
        .args(["version", "rejoin", "--version", "v1.2.0", "--dry-run"])
        .env("FORGEJO_TOKEN", "fixture")
        .env("FORGEJO_URL", url)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    server.join().unwrap();
}

#[test]
fn proof() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    super::marker::seeded(&fixture, bare.path());
    let dry = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "release",
            "stamp",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ])
        .output()
        .expect("dry release stamp");
    assert!(!dry.status.success());
    assert!(
        String::from_utf8_lossy(&dry.stderr).contains("unproved tree"),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
}
