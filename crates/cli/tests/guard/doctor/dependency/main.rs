use plumb_cli::{Dependency, Ecosystem, Verdict};
use std::path::Path;

mod npm;

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

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("seat should be made");
    std::fs::write(path, text).expect("fixture should be written");
}

fn workspace(root: &Path) {
    write(
        root,
        "package.json",
        r#"{"name":"root","private":true,"devDependencies":{"@perishlab/crest":"0"}}"#,
    );
    write(
        root,
        "packages/design/package.json",
        r#"{"name":"@perishlab/design","dependencies":{"@perishlab/sign":"0","@perishlab/token":"^1.0.0"}}"#,
    );
    write(
        root,
        "packages/sign/package.json",
        r#"{"name":"@perishlab/sign"}"#,
    );
    write(
        root,
        "apps/docs/package.json",
        r#"{"name":"docs","dependencies":{"@perishlab/design":"workspace:*","@perishlab/sidecar":"0","svelte":"5"}}"#,
    );
}

#[test]
fn pinned() {
    let fixture = super::fixture();
    let root = fixture.path();
    workspace(root);
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    for expected in [
        "npm @perishlab/token requires ^1.0.0, expected 0 [packages/design/package.json]",
        "npm @perishlab/sign requires 0, expected workspace:* [packages/design/package.json]",
    ] {
        assert!(out.contains(expected), "{expected}\n{out}");
    }
    for held in ["crest", "design", "sidecar"] {
        assert!(
            !out.contains(&format!("@perishlab/{held} requires")),
            "{out}"
        );
    }
    assert!(!out.contains("svelte requires"), "{out}");
    assert!(out.contains("blind: cannot read npm lock"), "{out}");
}

#[test]
fn illegible() {
    let fixture = super::fixture();
    let root = fixture.path();
    workspace(root);
    write(root, "apps/docs/package.json", "{");
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(
        out.contains("blind: cannot read npm manifest apps/docs/package.json"),
        "{out}"
    );
    assert!(!out.contains("true to the skeleton"), "{out}");
}
