use plumb::config;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, PartialEq, Debug)]
struct Sample {
    name: String,
    count: u32,
}

#[test]
fn loads() {
    let dir = std::env::temp_dir().join("plumb-load");
    std::fs::create_dir_all(&dir).expect("fixture");
    let path = dir.join("sample.toml");
    std::fs::write(&path, "name = \"x\"\ncount = 3\n").expect("write");
    let got: Sample = config::load(&path).expect("load");
    std::fs::remove_dir_all(&dir).expect("sweep");
    assert_eq!(
        got,
        Sample {
            name: "x".to_string(),
            count: 3
        }
    );
}

#[test]
fn missing() {
    let err = config::load::<Sample>(&PathBuf::from("/no/such-xyz.toml")).unwrap_err();
    assert!(err.to_string().contains("cannot read"), "{err}");
}

#[test]
fn malformed() {
    let dir = std::env::temp_dir().join("plumb-bad");
    std::fs::create_dir_all(&dir).expect("fixture");
    let path = dir.join("bad.toml");
    std::fs::write(&path, "name = \n").expect("write");
    let err = config::load::<Sample>(&path).unwrap_err();
    std::fs::remove_dir_all(&dir).expect("sweep");
    assert!(err.to_string().contains("cannot parse"), "{err}");
}

#[test]
fn finds() {
    let root = std::env::temp_dir().join("plumb-find");
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).expect("fixture");
    std::fs::write(root.join("target.toml"), "").expect("write");
    let found = config::discover(&nested, "target.toml").expect("found");
    std::fs::remove_dir_all(&root).expect("sweep");
    assert_eq!(found, root.join("target.toml"));
}

#[test]
fn absent() {
    let root = std::env::temp_dir().join("plumb-absent");
    std::fs::create_dir_all(&root).expect("fixture");
    let searched = config::discover(&root, "nope-xyz.toml").unwrap_err();
    std::fs::remove_dir_all(&root).expect("sweep");
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
    for key in ["GIT_DIR", "GIT_INDEX_FILE"] {
        let held = command
            .get_envs()
            .find(|(held, _)| *held == key)
            .unwrap_or_else(|| panic!("{key} is explicitly cleared"));
        assert!(held.1.is_none());
    }
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
