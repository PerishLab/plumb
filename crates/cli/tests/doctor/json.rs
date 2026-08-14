use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn run(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", "--json", root.to_str().expect("utf8 root")])
        .output()
        .expect("run plumb doctor")
}

#[test]
fn clean() {
    let fixture = crate::fixture();
    let output = run(fixture.path());
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["operation"], "doctor");
    assert_eq!(report["target"], fixture.path().display().to_string());
    assert_eq!(report["ok"], true);
    assert_eq!(report["clean"], true);
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["summary"]["out_of_true"], 0);
    assert_eq!(report["summary"]["unknown"], 0);
    assert_eq!(report["summary"]["blind"], 0);
    assert_eq!(report["coverage"]["mechanized"], 72);
    assert_eq!(report["coverage"]["observed"], 1);
    assert_eq!(report["coverage"]["prose_only"], 40);
    assert!(report["shape"]["wrappers"].is_array());
    assert!(report["shape"]["layout"].is_array());
    assert!(report["shape"]["documents"].is_array());
    assert_eq!(report["vocabulary"]["schema"], "plumb.vocabulary/v1");
    assert_eq!(report["vocabulary"]["codec"], "p64-v1");
    assert!(
        report["vocabulary"]["dictionary_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(report["vocabulary"]["retired"], 1);
    assert_eq!(report["vocabulary"]["coverage"]["tracked"], 0);
    assert_eq!(report["vocabulary"]["hits"], serde_json::json!([]));
}

#[test]
fn failing() {
    let fixture = crate::fixture();
    std::fs::write(fixture.path().join("runseal.toml"), "").expect("create governed fixture");
    let output = run(fixture.path());
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["ok"], false);
    assert_eq!(report["clean"], false);
    let findings = report["findings"].as_array().expect("findings");
    assert!(!findings.is_empty());
    for finding in findings {
        let code = finding["code"].as_str().expect("finding code");
        let scope = finding["scope"].as_str().expect("finding scope");
        assert!(code.starts_with(&format!("{scope}.")), "{code}");
        assert!(!finding["evidence"].as_str().unwrap_or("").is_empty());
        assert_eq!(finding["standing"], "mechanized");
        assert!(finding["owner"].is_string());
        assert!(
            finding["tags"]
                .as_array()
                .is_some_and(|tags| !tags.is_empty())
        );
    }
    assert!(
        findings
            .iter()
            .any(|finding| finding["code"] == "structure.guard-lane-present")
    );
}

#[test]
fn unknown() {
    let fixture = crate::fixture();
    std::fs::create_dir(fixture.path().join("novel")).expect("create unknown shape");
    let empty = run(fixture.path());
    let empty: Value = serde_json::from_slice(&empty.stdout).expect("doctor json");
    assert_eq!(empty["summary"]["unknown"], 0);
    std::fs::write(fixture.path().join("novel/held.txt"), "held\n").expect("tracked shape");
    let status = Command::new("git")
        .arg("-C")
        .arg(fixture.path())
        .args(["add", "novel/held.txt"])
        .status()
        .expect("git add");
    assert!(status.success());
    let output = run(fixture.path());
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["ok"], true);
    assert_eq!(report["clean"], false);
    assert_eq!(report["summary"]["unknown"], 1);
    assert_eq!(report["findings"][0]["code"], "structure.known-directory");
    assert_eq!(report["findings"][0]["grade"], "unknown shape");
}

#[test]
fn evidence() {
    let fixture = crate::fixture();
    let output = run(fixture.path());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["shape"]["dependencies"], serde_json::json!([]));
}

#[test]
fn blind() {
    let fixture = crate::fixture();
    std::fs::write(fixture.path().join("ectropy.toml"), "{").expect("malformed policy");
    let output = run(fixture.path());
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["ok"], false);
    assert_eq!(report["summary"]["out_of_true"], 0);
    assert_eq!(report["summary"]["blind"], 1);
}
