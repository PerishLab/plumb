use serde_json::Value;
use std::process::{Command, Output};

const WORDS: &str = r#"
[[owner]]
id = "release"
summary = "fixture"

[[owner]]
id = "skill"
summary = "fixture"

[[tag]]
id = "site"
summary = "fixture"

[[tag]]
id = "cargo"
summary = "fixture"

[[namespace]]
id = "site"
summary = "fixture"
owner = "release"

[[namespace]]
id = "skill"
summary = "fixture"
owner = "skill"
"#;
const RULES: &str = r#"
[[rule]]
id = "site.notes-published"
summary = "fixture"
law = "fixture"
evidence = "fixture"
standing = "prose-only"
owner = "release"
tags = ["site"]

[[rule]]
id = "site.crate-documented"
summary = "fixture"
law = "fixture"
evidence = "fixture"
standing = "prose-only"
owner = "release"
tags = ["site", "cargo"]

[[rule]]
id = "skill.seat-matches-binary"
summary = "fixture"
law = "fixture"
evidence = "fixture"
standing = "observed"
owner = "skill"
tags = ["site"]
"#;

fn seat() -> tempfile::TempDir {
    let base = crate::support::depot(&[]);
    let words = crate::support::object(base.path(), "rules/taxonomy.toml") + WORDS;
    let law = crate::support::object(base.path(), "rules/catalog.toml") + RULES;
    crate::support::depot(&[
        ("rules/taxonomy.toml", &words),
        ("rules/catalog.toml", &law),
    ])
}

fn run(args: &[&str]) -> Output {
    let home = seat();
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
    assert_eq!(rule["owner"], "fixture");
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
    assert_eq!(rules.len(), 1, "{rules:?}");
    assert_eq!(rules[0]["id"], "site.notes-published");
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
    let observed = json(&["rule", "list", "--standing", "observed", "--json"]);
    let prose = json(&["rule", "list", "--standing", "prose-only", "--json"]);
    assert_eq!(all["schema"], "plumb.rule-list/v1");
    let total = all["rules"].as_array().map(Vec::len).expect("all rules");
    let home = seat();
    let law = crate::support::object(home.path(), "rules/catalog.toml");
    assert_eq!(total, law.matches("[[rule]]").count());
    let classified = [&mechanized, &observed, &prose]
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
