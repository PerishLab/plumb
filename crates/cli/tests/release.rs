use plumb::semantics::{Baseline, Coverage, Kind, Material, Release, Request, Snapshot, Source};
use std::process::Command;

fn request() -> Request {
    let snapshot = Snapshot {
        product: "plumb".into(),
        source: Source {
            repository: "PerishLab/plumb".into(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
        },
        materials: vec![
            Material::new(Kind::Schema, "api.json".into(), b"{}", false).unwrap(),
            Material::new(Kind::Case, "writer".into(), b"assert behavior", true).unwrap(),
        ],
        coverage: vec![Coverage {
            case: "writer".into(),
            platform: "linux-x86_64".into(),
            backend: "none".into(),
        }],
    };
    let release = Release {
        marker: "v0.28.2".into(),
        snapshot: snapshot.clone(),
    };
    Request {
        schema: Request::SCHEMA.into(),
        baseline: Baseline {
            stable: release.clone(),
            anchor: release,
            review: None,
        },
        candidate: snapshot,
        review: None,
        reservations: vec![],
        major: None,
    }
}

fn invoke(bytes: &[u8]) -> (i32, String, String) {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("input.json");
    std::fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "assess", "--input"])
        .arg(&path)
        .env("PLUMB_HOME", scratch.path())
        .output()
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 1);
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn advisory() {
    let (code, body, error) = invoke(&serde_json::to_vec(&request()).unwrap());
    assert_eq!(code, 0, "{error}");
    let report: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["schema"], "plumb.release.assessment/v1");
    assert_eq!(report["state"], "same");
    assert_eq!(report["marker"], "v0.28.3");
    assert_eq!(report["authentication"], false);
    assert_eq!(report["publication"], false);
}

#[test]
fn unknown() {
    let mut held = request();
    held.candidate.source.commit = "c".repeat(40);
    let (code, body, error) = invoke(&serde_json::to_vec(&held).unwrap());
    assert_eq!(code, 3, "{error}");
    let report: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["state"], "unknown");
    assert!(report["marker"].is_null());
}

#[test]
fn conflict() {
    let mut held = request();
    held.reservations.push("v0.29.0-rc.1".into());
    let (code, body, error) = invoke(&serde_json::to_vec(&held).unwrap());
    assert_eq!(code, 1, "{error}");
    let report: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["state"], "conflict");
    assert!(report["marker"].is_null());
}

#[test]
fn invalid() {
    let (code, body, error) = invoke(br#"{"schema":"foreign"}"#);
    assert_eq!(code, 1);
    assert!(body.is_empty());
    assert!(error.contains("invalid release semantics"), "{error}");
}
