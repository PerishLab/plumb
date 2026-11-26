use clap::Parser as _;
use std::collections::BTreeSet;

#[test]
fn consign() {
    let hint = super::consign("v1.0.0");
    crate::Cli::try_parse_from(hint.split_whitespace()).unwrap_or_else(|error| {
        panic!("the changelog hint names no Plumb command: {hint}\n{error}")
    });
}

#[test]
fn prune() {
    let mut doc: toml::Table = toml::from_str(
        "[[grant]]\nsyntax = \"test\"\npaths = [\"a/**\"]\n\n[[grant]]\nsyntax = \"environment\"\npaths = [\"crates/*/src/**/*.rs\", \"apps/web/vite.config.ts\"]\n\n[[grant]]\nsyntax = \"lock\"\npaths = [\"crates/cli/src/lib.rs\"]\n",
    )
    .expect("table");
    let covered = BTreeSet::from(["crates/*/src/**/*.rs".to_string()]);
    super::policy::prune(&mut doc, "grant", Some("test"), ("environment", &covered));
    let rendered = toml::to_string_pretty(&doc).expect("render");
    assert!(!rendered.contains("syntax = \"test\""), "{rendered}");
    assert!(!rendered.contains("crates/*/src/**/*.rs"), "{rendered}");
    assert!(rendered.contains("apps/web/vite.config.ts"), "{rendered}");
    assert!(rendered.contains("syntax = \"lock\""), "{rendered}");

    let mut fully: toml::Table =
        toml::from_str("[[grant]]\nsyntax = \"environment\"\npaths = [\"crates/*/src/**/*.rs\"]\n")
            .expect("table");
    super::policy::prune(&mut fully, "grant", Some("test"), ("environment", &covered));
    assert!(fully.is_empty(), "{fully:?}");
}
