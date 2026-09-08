use super::super::world::Fixture;
use super::super::world::run;
use serde_json::{Value, json};
use std::process::Command;

pub(super) fn settlement(command: &impl Fn() -> Command, graph: &Value) {
    use sha2::{Digest, Sha256};
    let root = command().get_current_dir().unwrap().to_path_buf();
    let git = |args: &[&str]| {
        String::from_utf8(run(Command::new("git").arg("-C").arg(&root).args(args)).stdout)
    };
    let commit = git(&["rev-parse", "v1.2.0^{commit}"])
        .unwrap()
        .trim()
        .to_string();
    let parent = git(&["rev-parse", "v1.2.0^{}^1"])
        .unwrap()
        .trim()
        .to_string();
    let marker = run(command().args(["release", "verify", "--marker", "v1.2.0", "--held"]));
    let marker = String::from_utf8(marker.stdout).unwrap();
    let marker = marker
        .rsplit_once('(')
        .unwrap()
        .1
        .trim()
        .trim_end_matches(')');
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(marker, "oci://registry.test/owner/probe")).unwrap())
    );
    let store = crate::support::Bucket::open(28);
    store.seed(&format!("records/binding/{key}.json"), &serde_json::to_vec(&json!({
        "action":"ship/oci", "binding":key, "workload":"0".repeat(64),
        "proof":"0".repeat(64), "publication":"0".repeat(64),
        "source":{"type":"url", "source":format!("https://registry.test/v2/owner/probe/manifests/sha256:{}", "a".repeat(64))},
    })).unwrap());
    let source = format!("{}/workflow/inventory.json", store.endpoint());
    let command = || {
        let mut held = command();
        held.env("PLUMB_WORKFLOW_INVENTORY_URL", &source)
            .env_remove("FORGEJO_TOKEN")
            .env_remove("FORGEJO_TOKEN_FILE");
        held
    };
    let inventory = root.join("depot/inventory.json");
    for (status, exit) in [("503", "0"), ("000", "7")] {
        let refused = command()
            .args(["version", "rejoin", "--version", "v1.2.0", "--dry-run"])
            .env("FAKE_INVENTORY_STATUS", status)
            .env("FAKE_INVENTORY_EXIT", exit)
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("cannot fetch Ship inventory"),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
    }
    let previous = std::fs::read(&inventory).unwrap();
    std::fs::write(
        &inventory,
        json!({"schema":"plumb.workflow-inventory/v1", "records":[]}).to_string(),
    )
    .unwrap();
    let rejoin = || {
        command()
            .args(["version", "rejoin", "--version", "v1.2.0", "--dry-run"])
            .output()
            .unwrap()
    };
    let refused = rejoin();
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("complete stable Ship graph"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    let unshipped = command()
        .args(["version", "prepare", "--version", "v1.3.0", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&unshipped.stderr).contains("FORGEJO_TOKEN"),
        "{}",
        String::from_utf8_lossy(&unshipped.stderr)
    );
    let cargo = graph["publication"]["include"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["request"])
        .find(|row| row["action"] == "ship/cargo")
        .unwrap();
    std::fs::write(
        &inventory,
        json!({"schema":"plumb.workflow-inventory/v1", "records":[{
            "action":"ship/cargo", "workload":cargo["keys"]["workload"],
            "proof":cargo["keys"]["proof"], "publication":cargo["keys"]["publication"],
            "source":{"type":"url", "source":"https://registry.test/probe"},
        }]})
        .to_string(),
    )
    .unwrap();
    git(&["update-ref", "refs/remotes/origin/main", &commit]).unwrap();
    let manifest = root.join("plumb.toml");
    let native = std::fs::read(&manifest).unwrap();
    std::fs::write(&manifest, "not valid TOML [[").unwrap();
    for _ in 0..2 {
        accepted(&command, &commit);
        assert_eq!(std::fs::read(&manifest).unwrap(), b"not valid TOML [[");
        assert_eq!(git(&["rev-parse", "HEAD"]).unwrap().trim(), commit);
    }
    git(&["update-ref", "refs/remotes/origin/main", &parent]).unwrap();
    for action in ["prepare", "freeze"] {
        let refused = command()
            .args(["version", action, "--version", "v1.3.0", "--dry-run"])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("which origin/main does not hold"),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
    }
    git(&["update-ref", "refs/remotes/origin/main", &commit]).unwrap();
    let settled = command()
        .args(["version", "prepare", "--version", "v1.3.0", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&settled.stderr).contains("FORGEJO_TOKEN"),
        "{}",
        String::from_utf8_lossy(&settled.stderr)
    );
    store.finish();
    let unread = rejoin();
    assert!(!unread.status.success());
    assert!(
        String::from_utf8_lossy(&unread.stderr).contains("cannot fetch public bucket object"),
        "{}",
        String::from_utf8_lossy(&unread.stderr)
    );
    let unread = command()
        .args(["version", "prepare", "--version", "v1.3.0", "--dry-run"])
        .output()
        .unwrap();
    assert!(!unread.status.success());
    assert!(
        String::from_utf8_lossy(&unread.stderr).contains("cannot fetch public bucket object"),
        "{}",
        String::from_utf8_lossy(&unread.stderr)
    );
    std::fs::write(&inventory, previous).unwrap();
    std::fs::write(&manifest, native).unwrap();
    git(&["status", "--porcelain"]).unwrap();
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
