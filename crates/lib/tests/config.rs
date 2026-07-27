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
