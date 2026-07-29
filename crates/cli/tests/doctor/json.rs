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
    let fixture = tempfile::tempdir().expect("fixture");
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
    assert_eq!(report["coverage"]["mechanized"], 75);
    assert_eq!(report["coverage"]["observed"], 0);
    assert_eq!(report["coverage"]["prose_only"], 28);
    assert!(report["shape"]["wrappers"].is_array());
    assert!(report["shape"]["layout"].is_array());
}

#[test]
fn failing() {
    let fixture = tempfile::tempdir().expect("fixture");
    std::fs::create_dir_all(fixture.path().join(".runseal/wrappers"))
        .expect("create governed fixture");
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
            .any(|finding| finding["code"] == "structure.missing-wrapper")
    );
}

#[test]
fn unknown() {
    let fixture = tempfile::tempdir().expect("fixture");
    std::fs::create_dir(fixture.path().join("novel")).expect("create unknown shape");
    let output = run(fixture.path());
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["ok"], true);
    assert_eq!(report["clean"], false);
    assert_eq!(report["summary"]["unknown"], 1);
    assert_eq!(report["findings"][0]["code"], "structure.known-directory");
    assert_eq!(report["findings"][0]["grade"], "unknown shape");
}
