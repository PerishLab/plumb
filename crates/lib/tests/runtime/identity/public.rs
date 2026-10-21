use plumb::lane::evidence::{Record, Reference, public::Reader};
use plumb::lane::{Atom, Digest};
use serde_json::{Value, json};

fn document() -> Value {
    let requested = json!({"repository":"PerishLab/crest","app":"crest","lane":"design"});
    let context =
        json!({"requested":requested,"request":"b".repeat(64),"revision":2,"adaptor":"binary"});
    let observation = json!({"outcome":"unknown","reason":"unverified"});
    let value = json!({"policy":"exact","context":context,"observation":observation});
    json!({"schema":2,"entry":{"kind":"selection","value":value}})
}

fn reader(bytes: &[u8]) -> Reader {
    Reader::new(
        "https://evidence.example.com",
        Reference::new(
            Atom::read("wharf").unwrap(),
            Digest::read(&plumb::depot::sha(bytes)).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn origins() {
    let reference = Reference::new(
        Atom::read("wharf").unwrap(),
        Digest::read(&"a".repeat(64)).unwrap(),
    )
    .unwrap();
    for origin in [
        "http://example.com",
        "https://example.com/",
        "https://user@example.com",
        "https://example.com:443",
        "https://example.com?key=value",
        "https://example.com#x",
        "https://EXAMPLE.com",
        "https://example.com.",
        "https://localhost",
        "https://example.local",
        "https://127.0.0.1",
        "https://-x.example.com",
        "https://example..com",
        "https://éxample.com",
    ] {
        assert!(Reader::new(origin, reference.clone()).is_err(), "{origin}");
    }
    let reader = Reader::new("https://pub-123.r2.dev", reference).unwrap();
    assert_eq!(
        reader.url(),
        format!(
            "https://pub-123.r2.dev/lane/v1/evidence/{}.json",
            "a".repeat(64)
        )
    );
}

#[test]
fn uncertainty() {
    let record: Record = serde_json::from_value(document()).unwrap();
    let bytes = record.encode().unwrap();
    let verified = reader(&bytes).verify(&bytes).unwrap();
    assert_eq!(verified.record(), &record);
    assert_eq!(
        serde_json::to_value(verified.record()).unwrap()["entry"]["value"]["observation"],
        json!({"outcome":"unknown","reason":"unverified"})
    );
    assert_eq!(verified.reference().digest(), &record.digest().unwrap());
    assert!(reader(&bytes).verify(b"{}").is_err());
}

#[test]
fn representation() {
    let mut value = document();
    value["entry"]["value"]["observation"] = json!({"outcome":"matched",
        "actual":{"repository":"PerishLab/crest","app":"crest","lane":"design"},
        "artifact":{"digest":"a".repeat(64),"source":{"repository":"PerishLab/crest",
        "commit":"c".repeat(40),"tree":"d".repeat(40)},"build":null}});
    let record: Record = serde_json::from_value(value).unwrap();
    let bytes = record.encode().unwrap();
    let verified = reader(&bytes).verify(&bytes).unwrap();
    assert_eq!(verified.record(), &record);
    assert!(!String::from_utf8(bytes).unwrap().contains("url"));
}

#[test]
fn canonical() {
    let pretty = serde_json::to_vec_pretty(&document()).unwrap();
    assert!(reader(&pretty).verify(&pretty).is_err());
    for field in ["schema", "private"] {
        let mut value = document();
        value[field] = json!("private-provider-value");
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = reader(&bytes).verify(&bytes).unwrap_err();
        assert!(!error.contains("private-provider-value"));
    }
    let bytes = vec![b'x'; plumb::lane::evidence::public::LIMIT + 1];
    assert!(
        reader(&bytes)
            .verify(&bytes)
            .unwrap_err()
            .contains("budget")
    );
    assert!(reader(b"").verify(b"").is_err());
}
