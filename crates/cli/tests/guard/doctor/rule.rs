use serde_json::Value;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    let home = crate::support::home();
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
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
    let report = json(&["rule", "show", "structure.anchor-present", "--json"]);
    assert_eq!(report["schema"], "plumb.rule/v1");
    let rule = &report["rule"];
    assert_eq!(rule["id"], "structure.anchor-present");
    assert_eq!(rule["namespace"], "structure");
    assert_eq!(rule["name"], "anchor-present");
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
    let law: toml::Table = crate::support::policy("rules/catalog.toml")
        .parse()
        .expect("catalog source");
    let held = law["rule"].as_array().expect("catalog rules");
    let field = |rule: &toml::Value, key: &str| rule[key].as_str().expect(key).to_string();
    let tags = |rule: &toml::Value| {
        rule["tags"]
            .as_array()
            .expect("tags")
            .iter()
            .map(|tag| tag.as_str().expect("tag").to_string())
            .collect::<Vec<_>>()
    };
    let pick = &held[0];
    let id = field(pick, "id");
    let namespace = id
        .split_once('.')
        .expect("qualified identity")
        .0
        .to_string();
    let standing = field(pick, "standing");
    let owner = field(pick, "owner");
    let tag = tags(pick)[0].clone();
    let mut expected = held
        .iter()
        .filter(|rule| field(rule, "id").starts_with(&format!("{namespace}.")))
        .filter(|rule| field(rule, "standing") == standing)
        .filter(|rule| field(rule, "owner") == owner)
        .filter(|rule| tags(rule).contains(&tag))
        .map(|rule| field(rule, "id"))
        .collect::<Vec<_>>();
    expected.sort();
    let report = json(&[
        "rule",
        "list",
        "--namespace",
        &namespace,
        "--standing",
        &standing,
        "--tag",
        &tag,
        "--owner",
        &owner,
        "--json",
    ]);
    let mut listed = report["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .map(|rule| rule["id"].as_str().expect("id").to_string())
        .collect::<Vec<_>>();
    listed.sort();
    assert_eq!(listed, expected);
}

#[test]
fn catalog() {
    let all = json(&["rule", "list", "--json"]);
    let mechanized = json(&["rule", "list", "--standing", "mechanized", "--json"]);
    let observed = json(&["rule", "list", "--standing", "observed", "--json"]);
    let prose = json(&["rule", "list", "--standing", "prose-only", "--json"]);
    let retired = json(&["rule", "list", "--standing", "retired", "--json"]);
    assert_eq!(all["schema"], "plumb.rule-list/v1");
    let total = all["rules"].as_array().map(Vec::len).expect("all rules");
    let law = crate::support::policy("rules/catalog.toml");
    assert_eq!(total, law.matches("[[rule]]").count());
    let classified = [&mechanized, &observed, &prose, &retired]
        .iter()
        .map(|report| report["rules"].as_array().map(Vec::len).expect("rules"))
        .sum::<usize>();
    assert_eq!(classified, total);

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

#[test]
fn briefs() {
    let listed = json(&["rule", "show", "skill.seat-matches-binary", "--json"]);
    assert_eq!(listed["rule"]["standing"], "observed");
    assert_eq!(listed["rule"]["owner"], "skill");
}
