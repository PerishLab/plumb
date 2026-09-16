use super::super::world::{Fixture, run};
use std::process::Command;
#[path = "../../../../src/command/ship/request/context.rs"]
mod context;

fn request() -> serde_json::Value {
    serde_json::json!({
        "schema": "plumb.ship-request/v3",
        "marker": "v1.2.0-beta.1",
        "action": "ship/produce.aarch64-apple-darwin",
        "operation": {
            "type": "produce", "target": "aarch64-apple-darwin", "archive": "probe.tar.gz",
        },
        "configuration": "a".repeat(64),
        "profile": "b".repeat(64),
    })
}

fn refuses(request: serde_json::Value, expected: &str) {
    let root = tempfile::tempdir().expect("isolated root");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .current_dir(root.path())
        .env("PLUMB_HOME", root.path())
        .args(["ship", "execute", "--request", &request.to_string()])
        .output()
        .expect("request validation");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(expected), "{error}");
}

#[test]
fn protocol() {
    let mut held = request();
    held["schema"] = serde_json::json!("plumb.ship-request/v2");
    refuses(held, "requires plumb.ship-request/v3");
}

#[test]
fn inventory() {
    for field in ["keys", "roots", "projections"] {
        let mut held = request();
        held[field] = serde_json::json!({});
        refuses(held, "unknown field");
    }
}

#[test]
fn explicit() {
    let mut held = request();
    held.as_object_mut().expect("request").remove("marker");
    refuses(held, "missing field `marker`");
}

#[test]
fn nested() {
    let mut held = request();
    held["operation"] = serde_json::json!({
        "type": "bind", "target": "aarch64-apple-darwin", "archive": "probe.tar.gz",
        "build": {
            "reuse": {"type": "workload", "source": "https://blob.example/object"},
            "production": "c".repeat(64), "receipt": null, "keys": {},
        },
    });
    refuses(held, "unknown field `keys`");
}

#[test]
fn input() {
    refuses(
        request(),
        "only production requires an explicit input snapshot",
    );
    let mut held = request();
    held["operation"] = serde_json::json!({"type": "cargo"});
    held["input"] = serde_json::json!("/unexpected/input");
    refuses(held, "only production requires an explicit input snapshot");
}

#[test]
fn context() {
    let payload = request();
    let mut held = serde_json::json!({
        "node": payload["action"], "payload": payload,
        "inputs": {"source": {"digest": "a".repeat(64)}},
        "materialized": {"source": {"key": "a".repeat(64), "root": "/isolated/input"}},
    });
    let prepared = context::request(&held).expect("verified input mapping");
    assert_eq!(prepared["input"], "/isolated/input");
    held["materialized"]["source"]["key"] = serde_json::json!("b".repeat(64));
    assert!(context::request(&held).is_err());
    held["node"] = serde_json::json!("different");
    assert!(context::request(&held).is_err());
}

#[test]
fn local() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let blind = tempfile::tempdir().expect("blind depot");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    super::marker::seeded(&fixture, bare.path());
    let manifest = std::fs::read_to_string(fixture.root.join("plumb.toml")).expect("manifest");
    std::fs::write(
        fixture.root.join("plumb.toml"),
        manifest.replace("product = \"probe\"", "product = \"plumb\""),
    )
    .expect("plumb controller");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["commit", "-qam", "identify plumb"]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "HEAD:refs/heads/release/v1.2.0",
    ]));
    let output = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args([
            "release",
            "stamp",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ])
        .output()
        .expect("release stamp");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!error.contains("plumb depot seat is unreadable"), "{error}");
    assert!(error.contains("unproved tree"), "{error}");

    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0-beta.1",
        "-m",
        "plumb v1.2.0-beta.1",
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "refs/tags/v1.2.0-beta.1",
    ]));
    let verified = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args(["release", "verify", "--marker", "v1.2.0-beta.1", "--held"])
        .output()
        .expect("release verify");
    assert!(!verified.status.success());
    let error = String::from_utf8_lossy(&verified.stderr);
    assert!(!error.contains("plumb depot seat is unreadable"), "{error}");
    assert!(error.contains("has no valid guard proof"), "{error}");

    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "set-url",
        "origin",
        "ssh://git@git.perish.top/PerishLab/plumb.git",
    ]));
    let governed = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args(["release", "verify", "--marker", "v1.2.0-beta.1", "--held"])
        .output()
        .expect("governed release verify");
    assert!(!governed.status.success());
    let error = String::from_utf8_lossy(&governed.stderr);
    assert!(
        error.contains("product identity git.perish.top/PerishLab/plumb is absent"),
        "{error}"
    );

    super::profile::catalogued(blind.path(), "central", None, "PerishLab/plumb");
    let marker = "v1.2.0-beta.2";
    let annotation = serde_json::json!({
        "schema": "plumb.release-marker/v4",
        "product": "plumb",
        "marker": marker,
        "datum": "0".repeat(64),
    });
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        marker,
        "-m",
        &annotation.to_string(),
    ]));
    let unbound = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args(["release", "verify", "--marker", marker, "--held"])
        .output()
        .expect("unbound release verify");
    assert!(!unbound.status.success());
    let error = String::from_utf8_lossy(&unbound.stderr);
    assert!(
        error.contains("omitted its Product Profile binding"),
        "{error}"
    );
}
