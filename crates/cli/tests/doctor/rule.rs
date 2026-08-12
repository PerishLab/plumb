use serde_json::Value;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("run plumb rule")
}

fn json(args: &[&str]) -> Value {
    let output = run(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("rule json")
}

#[test]
fn show() {
    let report = json(&["rule", "show", "structure.guard-lane-present", "--json"]);
    assert_eq!(report["schema"], "plumb.rule/v1");
    let rule = &report["rule"];
    assert_eq!(rule["id"], "structure.guard-lane-present");
    assert_eq!(rule["namespace"], "structure");
    assert_eq!(rule["name"], "guard-lane-present");
    assert_eq!(rule["standing"], "mechanized");
    assert_eq!(rule["owner"], "plumb");
    assert!(rule["law"].as_str().is_some_and(|value| !value.is_empty()));
    assert!(
        rule["evidence"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(rule["tags"].as_array().is_some_and(|tags| !tags.is_empty()));
}

#[test]
fn invalid() {
    for args in [
        vec!["rule", "show", "structure.not-a-rule"],
        vec!["rule", "list", "--namespace", "missing"],
        vec!["rule", "list", "--tag", "missing"],
        vec!["rule", "list", "--standing", "missing"],
        vec!["rule", "list", "--owner", "missing"],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unknown"),
            "{args:?}"
        );
    }
}

#[test]
fn select() {
    let report = json(&[
        "rule",
        "list",
        "--namespace",
        "site",
        "--standing",
        "prose-only",
        "--tag",
        "site",
        "--owner",
        "release",
        "--without-tag",
        "cargo",
        "--json",
    ]);
    let rules = report["rules"].as_array().expect("rules");
    assert!(!rules.is_empty());
    for rule in rules {
        assert_eq!(rule["namespace"], "site");
        assert_eq!(rule["standing"], "prose-only");
        assert_eq!(rule["owner"], "release");
        assert!(
            rule["tags"]
                .as_array()
                .expect("tags")
                .contains(&Value::String("site".to_string()))
        );
    }
}

#[test]
fn catalog() {
    let all = json(&["rule", "list", "--json"]);
    let mechanized = json(&["rule", "list", "--standing", "mechanized", "--json"]);
    let prose = json(&["rule", "list", "--standing", "prose-only", "--json"]);
    assert_eq!(all["schema"], "plumb.rule-list/v1");
    assert_eq!(all["rules"].as_array().map(Vec::len), Some(118));
    assert_eq!(mechanized["rules"].as_array().map(Vec::len), Some(77));
    assert_eq!(prose["rules"].as_array().map(Vec::len), Some(40));

    for (deed, schema, field) in [
        ("namespaces", "plumb.rule-namespaces/v1", "namespaces"),
        ("tags", "plumb.rule-tags/v1", "tags"),
        ("owners", "plumb.rule-owners/v1", "owners"),
    ] {
        let report = json(&["rule", deed, "--json"]);
        assert_eq!(report["schema"], schema);
        assert!(
            report[field]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        );
    }
}
