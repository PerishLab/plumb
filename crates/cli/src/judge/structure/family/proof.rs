use super::*;

#[test]
fn merged() {
    let mut found = BTreeMap::from([("cargo".into(), 2), ("npm".into(), 1)]);
    merge(
        &mut found,
        BTreeMap::from([("cargo".into(), 14), ("npm".into(), 1)]),
    );
    merge(&mut found, BTreeMap::from([("npm".into(), 4)]));
    assert_eq!(found.get("cargo").copied(), Some(14));
    assert_eq!(found.get("npm").copied(), Some(4));
}

#[test]
fn parsing() {
    let body = br#"{
        "marker":"v0.19.1",
        "commit":"dbfa7ab4f1bce015033b9afe33ae5b9f93ffe512",
        "state":"complete",
        "repository":"PerishLab/radicula",
        "media":{"cargo":"published","npm":"published","binaries":"none"}
    }"#;
    let held = parsed(body, "v0.19.1", "memory").expect("record");
    assert_eq!(held.state, "complete");
    assert_eq!(held.repository, "PerishLab/radicula");
    assert_eq!(
        held.media.get("cargo").map(String::as_str),
        Some("published")
    );
}

#[test]
fn counted() {
    let record = Record {
        state: "complete".into(),
        repository: "PerishLab/radicula".into(),
        commit: "deadbeef".into(),
        media: BTreeMap::from([
            ("cargo".into(), "published".into()),
            ("npm".into(), "published".into()),
            ("binaries".into(), "published".into()),
        ]),
    };
    let held = sections(
        "[release.cargo]\npackages = [\"a\", \"b\", \"c\"]\n[release.npm]\npackages = [\"x\"]\n",
    )
    .unwrap();
    assert_eq!(held.get("cargo").copied(), Some(3));
    assert_eq!(held.get("npm").copied(), Some(1));
    let packed: BTreeMap<_, _> = record
        .media
        .iter()
        .filter(|(name, state)| packaged(name) && *state == "published")
        .map(|(name, _)| (name.clone(), held.get(name).copied().unwrap_or(0)))
        .collect();
    assert_eq!(packed.get("cargo").copied(), Some(3));
    assert!(!packed.contains_key("binaries"));
}

#[test]
fn skipped() {
    let record = Record {
        state: "incomplete".into(),
        repository: "PerishLab/radicula".into(),
        commit: "deadbeef".into(),
        media: BTreeMap::from([("cargo".into(), "published".into())]),
    };
    assert!(super::counted(&record).unwrap().is_empty());
}

#[test]
fn latest() {
    let listing = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/v0.18.0
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\trefs/tags/v0.19.1^{}
cccccccccccccccccccccccccccccccccccccccc\trefs/tags/v0.19.1
dddddddddddddddddddddddddddddddddddddddd\trefs/tags/v0.19.1-rc.1^{}
";
    let held = plumb::land::rejoin::latest(plumb::land::rejoin::tags(listing)).unwrap();
    assert_eq!(held.marker, "v0.19.1");
    assert_eq!(held.commit, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
}

#[test]
fn trusted() {
    assert!(super::trusted("PerishLab/radicula"));
    assert!(!super::trusted("PerishLab/radicula/extra"));
    assert!(!super::trusted("https://example.com/x"));
    assert!(super::hashed("dbfa7ab"));
    assert!(!super::hashed("xyz"));
    assert!(!super::hashed("short"));
}
