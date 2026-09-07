use plumb::rule::{Fields, document};
use serde_json::json;

#[test]
fn platforms() {
    use plumb::rule::Probe;
    let rules: Vec<Probe> = serde_json::from_value(json!([
        {"argv":["tool","--version"],"stdout":"1", "platform":["linux-x86_64"]},
        {"argv":["tool.exe","--version"],"stdout":"1", "platform":["windows-x86_64"]}
    ]))
    .expect("rules");
    assert_eq!(
        Probe::select(&rules, "windows-x86_64")
            .expect("Windows")
            .argv[0],
        "tool.exe"
    );
    assert!(Probe::select(&rules, "unknown").is_err());
    let rules: Vec<Probe> = serde_json::from_value(json!([
        {"argv":["tool"],"stdout":"1"}, {"argv":["tool"],"stdout":"1", "platform":["linux-x86_64"]}
    ]))
    .expect("overlap");
    assert!(Probe::select(&rules, "linux-x86_64").is_err());
    for value in [
        json!({"argv":[],"stdout":"1"}),
        json!({"argv":["tool"],"stdout":"1","platform":[]}),
    ] {
        let probe: Probe = serde_json::from_value(value).expect("decode");
        assert!(probe.validate().is_err());
    }
    assert!(
        serde_json::from_value::<Probe>(json!({"argv":["tool"],"stdout":"1","shell":true}))
            .is_err()
    );
}

#[test]
fn output() {
    let mut output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .expect("git");
    assert!(output.status.success());
    let probe: plumb::rule::Probe =
        serde_json::from_value(json!({"argv":["git","--version"],"stdout":"tool 1\nnext"}))
            .expect("rule");
    output.stdout = b"tool 1\r\nnext\r\n".to_vec();
    assert!(probe.check(&output).expect("CRLF"));
    output.stdout = b"tool 1\nnext \n".to_vec();
    assert!(!probe.check(&output).expect("spaces remain significant"));
    output.stdout = vec![255];
    assert!(probe.check(&output).is_err());
    let failed = std::process::Command::new("git")
        .arg("--plumb-invalid-probe")
        .output()
        .expect("git");
    assert!(probe.check(&failed).is_err());
}

#[test]
fn fields() {
    let rule: Fields = toml::from_str("deny = ['packageManager']").expect("rule");
    for text in [
        r#"{"packageManager":"pnpm@11.13.0"}"#,
        "{\n  \"packageManager\" : null\n}",
    ] {
        let doc = document("package.json", text.as_bytes()).expect("document");
        assert_eq!(rule.check(&doc).expect("check").len(), 1);
    }
    assert!(
        rule.check(&json!({"description":"packageManager"}))
            .expect("check")
            .is_empty()
    );
}

#[test]
fn nested() {
    let rule: Fields =
        toml::from_str("pointer = '/scripts'\nallow = ['test']\nrequired = ['test']")
            .expect("rule");
    assert!(
        rule.check(&json!({"scripts":{"test":"anything"}}))
            .expect("check")
            .is_empty()
    );
    assert_eq!(
        rule.check(&json!({"scripts":{"build":"private-value"}}))
            .expect("check")
            .len(),
        2
    );
    assert_eq!(
        rule.check(&json!({"scripts":[]}))
            .expect("wrong shape")
            .len(),
        1
    );
    assert_eq!(rule.check(&json!({})).expect("absent object").len(), 1);
}

#[test]
fn dialects() {
    let rule: Fields =
        toml::from_str("pointer = '/package'\ndeny = ['rust-version']").expect("rule");
    let doc = document(
        "Cargo.toml",
        b"[package]\nname = 'probe'\nrust-version = '1.90'",
    )
    .expect("TOML");
    assert_eq!(rule.check(&doc).expect("check").len(), 1);
    assert!(document("manifest.yaml", b"{}").is_err());
    assert!(document("manifest.json", &[255]).is_err());
    assert!(document("manifest.json", b"{private-value").is_err());
}

#[test]
fn malformed() {
    assert!(toml::from_str::<Fields>("unknown = true").is_err());
    assert!(toml::from_str::<Fields>("deny = 'field'").is_err());
    for pointer in ["field", "/bad~", "/bad~2"] {
        let rule = Fields {
            pointer: pointer.into(),
            ..Fields::default()
        };
        assert!(rule.validate().is_err());
    }
    let rule = Fields {
        pointer: "/a~1b/~0".into(),
        deny: vec!["secret".into()],
        ..Fields::default()
    };
    let found = rule
        .check(&json!({"a/b":{"~":{"secret":"private-value"}}}))
        .expect("escaped");
    assert_eq!(found.len(), 1);
    assert!(!found[0].contains("private-value"));
}
