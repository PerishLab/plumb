use plumb_lib::config;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, PartialEq, Debug)]
struct Sample {
    name: String,
    count: u32,
}

#[test]
fn loads() {
    let dir = std::env::temp_dir().join("plumb-lib-load");
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
    assert!(err.contains("cannot read"), "{err}");
}

#[test]
fn malformed() {
    let dir = std::env::temp_dir().join("plumb-lib-bad");
    std::fs::create_dir_all(&dir).expect("fixture");
    let path = dir.join("bad.toml");
    std::fs::write(&path, "name = \n").expect("write");
    let err = config::load::<Sample>(&path).unwrap_err();
    std::fs::remove_dir_all(&dir).expect("sweep");
    assert!(err.contains("cannot parse"), "{err}");
}

#[test]
fn absent() {
    let err = config::discover("plumb-lib-absent-xyz.toml").unwrap_err();
    assert!(err.contains("no plumb-lib-absent-xyz.toml found"), "{err}");
}
