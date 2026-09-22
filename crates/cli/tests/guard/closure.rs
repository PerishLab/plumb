use serde_json::Value;
use sha2::{Digest, Sha256};
use std::process::{Command, Output};

const COMMANDS: [&str; 15] = [
    "doctor",
    "land",
    "guard",
    "radius",
    "policy",
    "skill",
    "rule",
    "changelog",
    "configuration",
    "layout",
    "cookbook",
    "affirm",
    "release",
    "ship",
    "depot",
];

fn plumb(args: &[&str]) -> Output {
    let seat = super::support::home();
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .env("PLUMB_LOCUS_ENABLED", "false")
        .env("PLUMB_HOME", seat.path())
        .output()
        .expect("plumb should run")
}

fn success(args: &[&str]) -> Output {
    let output = plumb(args);
    assert!(
        output.status.success(),
        "plumb {args:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn children(help: &str) -> Vec<String> {
    let Some((_, commands)) = help.split_once("Commands:\n") else {
        return Vec::new();
    };
    commands
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(char::is_whitespace))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect()
}

fn capture(path: Vec<String>, held: &mut Vec<u8>) {
    let mut args = path.iter().map(String::as_str).collect::<Vec<_>>();
    args.push("--help");
    let output = success(&args);
    held.extend_from_slice(path.join(" ").as_bytes());
    held.push(0);
    held.extend_from_slice(&output.stdout);
    held.push(0);
    let help = String::from_utf8(output.stdout).expect("help should be utf8");
    for child in children(&help) {
        let mut nested = path.clone();
        nested.push(child);
        capture(nested, held);
    }
}

#[test]
fn command() {
    let root = success(&["--help"]);
    let help = String::from_utf8(root.stdout).expect("help should be utf8");
    assert_eq!(children(&help), COMMANDS);

    let mut held = Vec::new();
    capture(Vec::new(), &mut held);
    assert_eq!(
        digest(&held),
        "b8392441e296bc73b8551c2d8beff345c5ed8626c46e5078bb09a8b9020a4086"
    );
}

#[test]
fn rule() {
    let seat = super::support::home();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["rule", "list", "--json"])
        .env("PLUMB_LOCUS_ENABLED", "false")
        .env("PLUMB_HOME", seat.path())
        .output()
        .expect("rule list");
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("rule list json");
    assert_eq!(report["schema"], "plumb.rule-list/v1");
    let catalog: toml::Table = super::support::policy("rules/catalog.toml")
        .parse()
        .expect("seat catalog");
    let mut rules = catalog["rule"]
        .as_array()
        .expect("catalog rules")
        .iter()
        .map(|rule| {
            let mut rule = serde_json::to_value(rule).expect("catalog rule");
            let id = rule["id"].as_str().expect("rule identity").to_string();
            let (namespace, name) = id.split_once('.').expect("qualified identity");
            rule["namespace"] = namespace.into();
            rule["name"] = name.into();
            rule
        })
        .collect::<Vec<_>>();
    rules.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    assert_eq!(report["rules"], serde_json::json!(rules));
}
