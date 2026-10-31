use super::{
    binary::Binary,
    material::Codec,
    world::{Fixture, GH},
};
use plumb::windtun::qualification::{Inspector, State};
use serde_json::json;

#[test]
fn archive() {
    let source = Binary::source("keel", "v1.0.0-rc.1");
    let bodies = [
        b"not a gzip".to_vec(),
        Binary::archive(&Binary::source("keel", "v1.0.0-rc.2")),
        Binary::pack(
            &source,
            &Binary::built(&source, Some("1111111111111111111111111111111111111111")),
        ),
    ];
    for raw in bodies {
        let mut fixture = Fixture::new();
        let authority = "https://releases.keel.perish.uk";
        let url = format!(
            "{authority}/v1/objects/sha256/{}/keel.tar.gz",
            Codec::sha(&raw)
        );
        let reference = Codec::reference(&url, &raw);
        fixture.bodies.insert(url, raw);
        let mut seal = fixture.pins["product"]["seal"].clone();
        seal["artifacts"]["linux-x64"] = reference;
        let url = seal["url"].as_str().unwrap().to_string();
        fixture.set(&url, &seal);
        let mut pointer = fixture.pins["product"]["pointer"].clone();
        pointer["seal"] = Codec::reference(&url, &Codec::bytes(&seal));
        fixture.set(&format!("{authority}/v1/channels/rc.json"), &pointer);
        fixture.pass["products"][0]["archive"] = seal["artifacts"]["linux-x64"]["sha256"].clone();
        fixture.pass["products"][0]["seal"] = json!(Codec::digest(&seal));
        fixture.publish();
        assert_eq!(
            Inspector::new(&fixture)
                .inspect("PerishLab/keel", "v1.0.0-rc.1")
                .state,
            State::Refused
        );
    }
}

#[test]
fn metadata() {
    let mut fixture = Fixture::new();
    let raw = fixture.bodies.get_mut(GH).unwrap();
    raw.pop();
    raw.extend_from_slice(b",\"visibility\":\"public\"}");
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn scene() {
    let mut fixture = Fixture::new();
    let url = fixture.pins["scene"]["manifest"]["url"]
        .as_str()
        .unwrap()
        .strip_suffix("manifest.json")
        .unwrap();
    let key = format!("{url}objects/keel/health.toml");
    fixture.bodies.get_mut(&key).unwrap()[0] = b'X';
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn bounds() {
    use plumb::depot::v3::{Manifest, Pointer, Publication};
    let mut fixture = Fixture::new();
    let url = fixture.pins["scene"]["manifest"]["url"].as_str().unwrap();
    let original = Manifest::parse(&fixture.bodies[url]).unwrap();
    let mut object = original.objects[0].clone();
    object.size = u64::MAX;
    let mut other = object.clone();
    other.path = "keel/other.toml".into();
    let manifest = Manifest::new(original.identity(), vec![object, other]).unwrap();
    let pointer = Pointer::new(
        &manifest,
        Publication {
            source: "https://depot.seatrial.perish.uk",
            prior: None,
            created: "2026-10-01T00:00:00Z".into(),
        },
    )
    .unwrap();
    fixture
        .bodies
        .insert(pointer.manifest.url.clone(), manifest.encode().unwrap());
    let value = serde_json::to_value(pointer).unwrap();
    fixture.set(
        "https://depot.seatrial.perish.uk/channels/beta/scenes/latest.json",
        &value,
    );
    fixture.pass["scene"] = value;
    fixture.publish();
    let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused);
}
