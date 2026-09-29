use super::{Authority, Distribution, Marker};

const BASE: &str = "https://releases.a.perish.uk";

fn authority() -> Authority {
    Authority::new(BASE).expect("authority")
}

fn marker(channel: &str, marker: &str) -> Option<Marker> {
    Some(Marker {
        channel: channel.into(),
        marker: marker.into(),
    })
}

#[test]
fn normalized() {
    assert_eq!(authority().base(), BASE);
    for refused in [
        "",
        "http://releases.a.perish.uk",
        "https://releases.a.perish.uk/",
        "https://releases.a .perish.uk",
    ] {
        assert_eq!(
            Authority::new(refused),
            Err("authority must be one normalized https URL".to_string()),
            "{refused}"
        );
    }
}

#[test]
fn declared() {
    let manifest = format!(
        "[release]\nproduct = \"a\"\nauthority = \"{BASE}\"\nbinaries = [\"a\"]\n[layout]\nnote = 1\n"
    );
    assert_eq!(Authority::declared(&manifest), Ok(Some(authority())));
    assert_eq!(Authority::declared("[layout]\nnote = 1\n"), Ok(None));
    assert_eq!(
        Authority::declared("[release]\nproduct = \"a\"\n"),
        Ok(None)
    );
    assert_eq!(
        Authority::declared("[release]\nauthority = \"\"\n"),
        Ok(None)
    );
    assert_eq!(
        Authority::declared("[release]\nauthority = \"https://a/\"\n"),
        Err("authority must be one normalized https URL".to_string())
    );
    assert!(Authority::declared("[release").is_err());
}

#[test]
fn layout() {
    let held = authority();
    assert_eq!(
        held.pointer("stable"),
        format!("{BASE}/v1/channels/stable.json")
    );
    assert_eq!(
        held.seal("beta", "v1.0.0-beta.1"),
        format!("{BASE}/v1/releases/beta/v1.0.0-beta.1/seal.json")
    );
    assert_eq!(
        held.distribution("stable", "v1.0.0"),
        format!("{BASE}/v1/releases/stable/v1.0.0/distribution.json")
    );
}

#[test]
fn recognised() {
    let held = authority();
    assert_eq!(
        held.record(&held.distribution("stable", "v1.0.0")),
        marker("stable", "v1.0.0")
    );
    assert_eq!(
        held.record(&format!(
            "{BASE}/v1/releases/rc/v2.0.0-rc.3/distribution.json"
        )),
        marker("rc", "v2.0.0-rc.3")
    );
}

#[test]
fn foreign() {
    let held = authority();
    for reference in [
        "https://releases.b.perish.uk/v1/releases/stable/v1.0.0/distribution.json",
        "https://releases.a.perish.uk.evil/v1/releases/stable/v1.0.0/distribution.json",
        "http://releases.a.perish.uk/v1/releases/stable/v1.0.0/distribution.json",
        "https://github.com/PerishLab/plumb/releases/tag/v1.0.0",
    ] {
        assert_eq!(held.record(reference), None, "{reference}");
    }
    let other = Authority::new("https://releases.a.perish.uk.evil").expect("authority");
    assert_eq!(held.record(&other.distribution("stable", "v1.0.0")), None);
}

#[test]
fn malformed() {
    let held = authority();
    for path in [
        "/v1/releases/stable/v1.0.0/seal.json",
        "/v1/releases/stable/v1.0.0/distribution.json.bak",
        "/v1/releases/stable/v1.0.0/x/distribution.json",
        "/v1/releases/v1.0.0/distribution.json",
        "/v1/releases//v1.0.0/distribution.json",
        "/v1/releases/stable//distribution.json",
        "/v2/releases/stable/v1.0.0/distribution.json",
        "/v1/channels/stable.json",
        "//v1/releases/stable/v1.0.0/distribution.json",
    ] {
        assert_eq!(held.record(&format!("{BASE}{path}")), None, "{path}");
    }
}

fn record(marker: &str, state: &str, npm: &str) -> Vec<u8> {
    format!(
        r#"{{"marker":"{marker}","commit":"abcdef","state":"{state}","extra":[1],"attempt":{{"run":"9","at":0}},"media":{{"oci":"none","npm":"{npm}","binaries":"published"}}}}"#
    )
    .into_bytes()
}

#[test]
fn parsed() {
    let held =
        Distribution::parse(&record("v1.0.0", "complete", "failed"), "v1.0.0").expect("record");
    assert!(held.complete());
    assert_eq!(held.commit, "abcdef");
    assert_eq!(held.run.as_deref(), Some("9"));
    assert_eq!(
        held.media.keys().collect::<Vec<_>>(),
        ["binaries", "npm", "oci"]
    );
    assert_eq!(held.public(), ["binaries"]);
    let open =
        Distribution::parse(&record("v1.0.0", "incomplete", "present"), "v1.0.0").expect("record");
    assert!(!open.complete());
    assert_eq!(open.public(), ["binaries", "npm"]);
    let bare = Distribution::parse(br#"{"marker":"v1.0.0"}"#, "v1.0.0").expect("record");
    assert!(!bare.complete());
    assert_eq!(bare.run, None);
    assert!(bare.public().is_empty());
}

#[test]
fn refused() {
    assert_eq!(
        Distribution::parse(&record("v0.9.0", "complete", "published"), "v1.0.0"),
        Err("v1.0.0 distribution record names \"v0.9.0\"".to_string())
    );
    assert!(
        Distribution::parse(b"{", "v1.0.0")
            .unwrap_err()
            .starts_with("v1.0.0 distribution record does not parse: ")
    );
}

#[test]
fn placed() {
    let placed = Authority::at("http://127.0.0.1:9/");
    assert_eq!(placed.base(), "http://127.0.0.1:9");
    assert_eq!(
        placed.pointer("stable"),
        "http://127.0.0.1:9/v1/channels/stable.json"
    );
    assert_eq!(Authority::at(BASE), authority());
    assert_eq!(
        Authority::at(&format!("{BASE}/")).seal("stable", "v1.0.0"),
        authority().seal("stable", "v1.0.0")
    );
}
