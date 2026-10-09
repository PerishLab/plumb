use plumb_cli::{Dependency, Ecosystem, Verdict};

fn specimen(ecosystem: Ecosystem, requirement: &str, resolution: &str) -> Dependency {
    Dependency {
        ecosystem,
        name: "plumb".into(),
        requirement: requirement.into(),
        pinned: false,
        resolution: resolution.into(),
        latest: Some("0.3.4".into()),
        seat: "Cargo.toml".into(),
    }
}

#[test]
fn cargo() {
    let held = specimen(Ecosystem::Cargo, "^0.3.0", "0.3.4");
    assert!(plumb_cli::judge(&held).is_empty());
    let stale = specimen(Ecosystem::Cargo, "^0.3", "0.3.3");
    assert!(matches!(
        plumb_cli::judge(&stale).as_slice(),
        [Verdict::Stale { .. }]
    ));
    let document: toml::Value = toml::from_str(
        r#"
        [workspace.dependencies]
        held = { version = "1", registry = "perish" }
        alias = { package = "renamed", version = "1", registry = "perish" }

        [target.'cfg(unix)'.dependencies]
        local = { path = "crates/local" }
        platform = { version = "1", registry = "perish" }
        "#,
    )
    .expect("manifest");
    assert_eq!(
        plumb_cli::packages(&document, "perish"),
        std::collections::BTreeSet::from(["held".into(), "platform".into(), "renamed".into()])
    );
}

#[test]
fn names() {
    let document: toml::Value = toml::from_str(
        r#"
        [dependencies]
        plain = "1"
        shared.workspace = true

        [dev-dependencies]
        alias = { package = "renamed", version = "1" }

        [workspace.dependencies]
        pooled = "1"

        [target.'cfg(windows)'.build-dependencies]
        platform = "1"
        "#,
    )
    .expect("manifest");
    assert_eq!(
        plumb_cli::names(&document),
        std::collections::BTreeSet::from([
            "plain".into(),
            "platform".into(),
            "pooled".into(),
            "renamed".into(),
            "shared".into(),
        ])
    );
}

#[test]
fn registry() {
    let index = br#"
{"vers":"0.18.1","yanked":false}
{"vers":"0.19.0-beta.1","yanked":false}
{"vers":"0.20.0","yanked":true}
{"vers":"0.6.0","yanked":false}
"#;
    assert_eq!(plumb_cli::cargo(index).expect("latest"), "0.18.1");
    assert_eq!(plumb_cli::route("a"), "1/a");
    assert_eq!(plumb_cli::route("ab"), "2/ab");
    assert_eq!(plumb_cli::route("abc"), "3/a/abc");
    assert_eq!(plumb_cli::route("plumb"), "pl/um/plumb");
}
