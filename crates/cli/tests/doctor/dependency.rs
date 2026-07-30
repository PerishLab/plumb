use plumb_cli::{Dependency, Ecosystem, Verdict};

fn specimen(ecosystem: Ecosystem, requirement: &str, resolution: &str) -> Dependency {
    Dependency {
        ecosystem,
        name: match ecosystem {
            Ecosystem::Jsr => "@perish/sealkit".into(),
            Ecosystem::Cargo => "plumb".into(),
        },
        requirement: requirement.into(),
        pinned: ecosystem == Ecosystem::Jsr && requirement != "*",
        resolution: resolution.into(),
        latest: Some("0.3.4".into()),
        seat: "Cargo.toml".into(),
    }
}

#[test]
fn deno() {
    let found = plumb_cli::judge(&specimen(Ecosystem::Jsr, "^0.3.1", "0.3.1"));
    assert_eq!(
        found,
        vec![
            Verdict::Pinned,
            Verdict::Stale {
                resolution: "0.3.1".into(),
                latest: "0.3.4".into(),
            },
        ]
    );
    assert_eq!(
        plumb_cli::specifier("jsr:@perish/sealkit", "@perish"),
        Some(("@perish/sealkit".into(), "*".into(), false))
    );
    assert_eq!(
        plumb_cli::specifier("jsr:@perish/sealkit@*", "@perish"),
        Some(("@perish/sealkit".into(), "*".into(), true))
    );
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
        "#,
    )
    .expect("manifest");
    assert_eq!(
        plumb_cli::packages(&document, "perish"),
        std::collections::BTreeSet::from(["held".into(), "renamed".into()])
    );
}

#[test]
fn registry() {
    assert_eq!(
        plumb_cli::jsr(br#"{"latest":"0.3.4"}"#).expect("latest"),
        "0.3.4"
    );
    assert!(plumb_cli::jsr(br#"{"latest":"0.3.4-beta.2"}"#).is_err());
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
