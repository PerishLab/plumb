use plumb::datum::{Datum, Tree};

#[test]
fn identity() {
    let temp = tempfile::tempdir().unwrap();
    let tree = Tree(temp.path());
    for reference in [
        "v1.2.0",
        "v1.2.0-beta.1",
        "v1.2.0-rc.2+build.7",
        "refs/heads/release/v1.2.0",
        "release/v1.2.0-beta.1",
    ] {
        assert_eq!(tree.line(reference), Some("v1.2.0".into()));
    }
    for reference in ["v1.2", "v1.2.0-beta..1", "v1.2.0-", "broken-beta.1"] {
        assert_eq!(tree.line(reference), Some(reference.into()));
    }
    let datum = Datum::new("v1.2.0", Vec::new());
    let path = tree.seat("v1.2.0");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, datum.encode().unwrap()).unwrap();
    let line = tree.line("v1.2.0-beta.1").unwrap();
    assert_eq!(tree.read(&line).unwrap(), Some(datum));
    assert!(tree.read("v1.2.0-beta.1").unwrap().is_none());
}
