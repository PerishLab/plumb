use plumb::config;
use radicula_config::Cascade;
use std::path::{Path, PathBuf};

#[test]
fn finds() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join("plumb-find");
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).expect("fixture");
    std::fs::write(root.join("target.toml"), "").expect("write");
    let found = config::discover(&nested, "target.toml").expect("found");
    assert_eq!(found, root.join("target.toml"));
}

#[test]
fn absent() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join("plumb-absent");
    std::fs::create_dir_all(&root).expect("fixture");
    let searched = config::discover(&root, "nope-xyz.toml").unwrap_err();
    assert!(
        searched.iter().any(|path| path.ends_with("nope-xyz.toml")),
        "{searched:?}"
    );
}

#[test]
fn rebases() {
    let base = Path::new("/etc/app");
    assert_eq!(
        config::rebase(Path::new("data.db"), base),
        base.join("data.db")
    );
    assert_eq!(
        config::rebase(Path::new("/abs/x"), base),
        PathBuf::from("/abs/x")
    );
}

#[test]
fn detached() {
    let command = config::detached("git");
    for key in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_QUARANTINE_PATH"] {
        let held = command
            .get_envs()
            .find(|(held, _)| *held == key)
            .unwrap_or_else(|| panic!("{key} is explicitly cleared"));
        assert!(held.1.is_none());
    }
}

#[test]
fn tool() {
    let root = tempfile::tempdir().expect("tool seat");
    let path = root.path().join("probe");
    std::fs::write(&path, "probe\n").expect("tool bytes");
    let tool = config::Tool::at(path.clone()).expect("fingerprinted tool");
    assert_eq!(tool.path, path);
    assert_eq!(
        tool.digest,
        "25be323556dad377abb57fe7ec8c4b99a6527f488dda28d0c9b686528659c909"
    );
    assert!(config::Tool::at(root.path().join("absent")).is_err());
}

#[test]
fn execution() {
    let contract: config::Contract =
        toml::from_str("inherit = ['PUBLIC']\nmanaged = ['OWNED']\nreject = ['BUILD_*']\n")
            .expect("contract");
    let values = [
        ("PUBLIC", "one"),
        ("OWNED", "ambient"),
        ("SECRET", "private"),
    ]
    .map(|(key, value)| (key.into(), value.into()));
    let held = contract.capture(values).expect("environment");
    assert_eq!(held.get("PUBLIC"), Some("one"));
    assert_eq!(held.get("SECRET"), None);
    assert_eq!(held.get("OWNED"), None);
    let encoded = serde_json::to_string(&held).expect("identity");
    assert!(!encoded.contains("private"));
    let mut command = std::process::Command::new("probe");
    command.env("OWNED", "governed");
    held.apply(&mut command);
    let keys = command.get_envs().collect::<Vec<_>>();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().any(|(key, value)| *key == "OWNED" && *value == Some(std::ffi::OsStr::new("governed"))));
    let refused = contract.capture([("BUILD_FLAGS".into(), "private".into())]);
    let error = refused.err().expect("refusal");
    assert!(error.contains("BUILD_FLAGS"));
    assert!(!error.contains("private"));
}

#[test]
fn homes() {
    let values = [
        ("RUSTUP_HOME", "/usr/local/rustup"),
        ("CARGO_HOME", "/usr/local/cargo"),
        ("PATH", "/usr/bin"),
    ]
    .map(|(key, value)| (key.into(), value.into()));
    let held = config::contract("probe")
        .expect("probe contract")
        .capture(values.clone())
        .expect("environment");
    assert_eq!(held.get("RUSTUP_HOME"), Some("/usr/local/rustup"));
    assert_eq!(held.get("CARGO_HOME"), Some("/usr/local/cargo"));
    let held = config::contract("pnpm")
        .expect("pnpm contract")
        .capture(values)
        .expect("environment");
    assert_eq!(held.get("RUSTUP_HOME"), None);
    assert_eq!(held.get("CARGO_HOME"), None);
}

#[test]
fn contract() {
    for text in [
        "inherit = ['PUBLIC']\nmanaged = ['PUBLIC']\nreject = []\n",
        "inherit = ['PUBLIC*']\nmanaged = []\nreject = []\n",
        "inherit = []\nmanaged = []\nreject = ['BUILD_*_FLAGS']\n",
    ] {
        let contract: config::Contract = toml::from_str(text).expect("contract");
        assert!(contract.capture([]).is_err());
    }
    assert!(toml::from_str::<config::Contract>("inherit = []").is_err());
    assert!(
        toml::from_str::<config::Contract>(
            "inherit = []\nmanaged = []\nreject = []\nunknown = true"
        )
        .is_err()
    );
}

#[test]
fn bound() {
    let contract: config::Contract = toml::from_str(
        "inherit=[]\nmanaged=[]\nreject=['BUILD_*']\n[bind]\nBUILD_INCREMENTAL='0'\n",
    )
    .unwrap();
    assert_eq!(
        contract.capture([]).unwrap().get("BUILD_INCREMENTAL"),
        Some("0")
    );
    assert!(
        contract
            .capture([("BUILD_INCREMENTAL".into(), "0".into())])
            .is_ok()
    );
    let error = contract
        .capture([("BUILD_INCREMENTAL".into(), "private-value".into())])
        .err()
        .unwrap();
    assert!(error.contains("BUILD_INCREMENTAL"));
    assert!(!error.contains("private-value"));
    for text in [
        "inherit=['X']\nmanaged=[]\nreject=[]\n[bind]\nX='0'",
        "inherit=[]\nmanaged=['X*']\nreject=[]\n[bind]\nX='0'",
        "inherit=[]\nmanaged=[]\nreject=[]\n[bind]\nX={tool='/bin/tool'}",
        "inherit=[]\nmanaged=[]\nreject=[]\n[bind]\nX={tool=''}",
    ] {
        let contract: config::Contract = toml::from_str(text).unwrap();
        assert!(contract.capture([]).is_err(), "{text}");
    }
    assert!(
        toml::from_str::<config::Contract>(
            "inherit=[]\nmanaged=[]\nreject=[]\n[bind]\nX={tool='tool', arbitrary='bad'}"
        )
        .is_err()
    );
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
struct Relay {
    key: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[plumb::config::prefix("FOLIO")]
struct Settings {
    issuer: String,
    #[cascade(section)]
    locus: Relay,
}

#[test]
fn migrated() {
    assert_eq!(Settings::prefix(), "FOLIO");
    let both = |key: &str| match key {
        "FOLIO_ISSUER" => Some("primary".to_string()),
        "API_ISSUER" => Some("legacy".to_string()),
        "FOLIO_LOCUS_KEY" => Some("   ".to_string()),
        "API_LOCUS_KEY" => Some("secret".to_string()),
        "API_OTHER" => Some("ignored".to_string()),
        "SIDECAR_PORT" => Some("9000".to_string()),
        _ => None,
    };
    let lookup = config::migrated(&Settings::prefix(), "API", &both);
    assert_eq!(lookup("FOLIO_ISSUER").as_deref(), Some("primary"));
    assert_eq!(lookup("FOLIO_LOCUS_KEY").as_deref(), Some("secret"));
    assert_eq!(lookup("FOLIOX_OTHER"), None);
    assert_eq!(lookup("SIDECAR_PORT").as_deref(), Some("9000"));
    assert_eq!(lookup("UNSET_KEY"), None);

    let resolved = Settings::resolve(None, &Settings::prefix(), &lookup).expect("resolved");
    assert_eq!(resolved.issuer, "primary");
    assert_eq!(resolved.locus.key, "secret");

    let fallback = |key: &str| match key {
        "API_ISSUER" => Some("legacy".to_string()),
        "API_LOCUS_KEY" => Some("legacy-key".to_string()),
        _ => None,
    };
    let merged = Settings::resolve(
        None,
        &Settings::prefix(),
        &config::migrated(&Settings::prefix(), "API", &fallback),
    )
    .expect("fallback");
    assert_eq!(merged.issuer, "legacy");
    assert_eq!(merged.locus.key, "legacy-key");
}
