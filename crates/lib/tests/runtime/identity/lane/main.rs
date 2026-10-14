mod admission;
mod artifact;
mod capability;
mod declaration;
mod evidence;
mod material;
mod operation;
mod projection;
mod receipt;

use plumb::lane::{Address, Atom, Name};
use serde_json::{Value, json};

fn address(lane: &str) -> Value {
    json!({"repository":"PerishLab/crest","app":"crest","lane":lane})
}

#[test]
fn names() {
    for text in [
        "a", "z", "a0", "design", "review", "preview", "lane", "a1b2",
    ] {
        let name = Name::read(text).unwrap();
        assert_eq!(name.value(), text);
        assert_eq!(serde_json::to_value(&name).unwrap(), json!(text));
        assert_eq!(serde_json::from_value::<Name>(json!(text)).unwrap(), name);
    }
    let text = "a".repeat(48);
    assert!(Name::read(&text).is_ok());
    assert!(serde_json::from_value::<Name>(json!(text)).is_ok());
}

#[test]
fn refusals() {
    for text in [
        "",
        "A",
        "Preview",
        "1",
        "1a",
        "a-b",
        "a_b",
        "a.b",
        "preview.a",
        "preview.",
        ".a",
        "preview.a.b",
        "preview..a",
        "preview/a",
        "preview\\a",
        "preview.%61",
        "preview.a_b",
        "preview.a--b",
        "preview.-a",
        "preview.a-",
        "preview.1a",
        "preview.é",
        "é",
        "aé",
        "a\n",
        "a\r",
        "a\0",
        " a",
        "a ",
        "v1.2.3-lane.a",
    ] {
        assert!(Name::read(text).is_err(), "{text:?}");
        assert!(serde_json::from_value::<Name>(json!(text)).is_err());
    }
    let text = "a".repeat(49);
    assert!(Name::read(&text).is_err());
    assert!(serde_json::from_value::<Name>(json!(text)).is_err());
    for value in [
        json!(null),
        json!(1),
        json!({"family":"preview"}),
        json!([]),
    ] {
        assert!(serde_json::from_value::<Name>(value).is_err());
    }
}

#[test]
fn alphabet() {
    for byte in 0_u8..=127 {
        let text = char::from(byte).to_string();
        assert_eq!(Name::read(&text).is_ok(), byte.is_ascii_lowercase());
        let text = format!("a{}", char::from(byte));
        assert_eq!(
            Name::read(&text).is_ok(),
            byte.is_ascii_lowercase() || byte.is_ascii_digit()
        );
    }
}

#[test]
fn atoms() {
    for text in ["a", "static", "wharf", "private-npm", "cf-worker", "a-10"] {
        let atom = Atom::read(text).unwrap();
        assert_eq!(atom.value(), text);
        assert_eq!(serde_json::to_value(&atom).unwrap(), json!(text));
        assert_eq!(serde_json::from_value::<Atom>(json!(text)).unwrap(), atom);
    }
    assert!(Atom::read(&"a".repeat(48)).is_ok());
    for text in ["", "A", "1a", "a.b", "a--b", "a-", "-a", "a_b", "a\n", "é"] {
        assert!(Atom::read(text).is_err());
        assert!(serde_json::from_value::<Atom>(json!(text)).is_err());
    }
    assert!(Atom::read(&"a".repeat(49)).is_err());
    assert!(Atom::read("private-npm").is_ok());
    assert!(Name::read("private-npm").is_err());
}

#[test]
fn targets() {
    let held = serde_json::from_value::<Address>(address("preview")).unwrap();
    assert_eq!(held.repository(), "PerishLab/crest");
    assert_eq!(held.app(), "crest");
    assert_eq!(held.lane().value(), "preview");
    assert_eq!(serde_json::to_value(&held).unwrap(), address("preview"));
    for lane in ["a", "design", "review"] {
        let target = serde_json::from_value::<Address>(address(lane)).unwrap();
        assert_ne!(target, held);
        assert_eq!(target.app(), held.app());
    }
    let name = Name::read("preview").unwrap();
    assert_eq!(
        Address::new("PerishLab/crest".into(), "crest".into(), name).unwrap(),
        held
    );
}

#[test]
fn shape() {
    for (field, value) in [
        ("repository", json!("../crest")),
        ("repository", json!("owner/repo/extra")),
        ("repository", json!("owner/r.epo")),
        ("repository", json!(format!("{}/crest", "a".repeat(101)))),
        ("app", json!("crest.review")),
        ("app", json!("1crest")),
        ("lane", json!("preview.a")),
        ("lane", json!("preview-a")),
        ("worker", json!("crest-review")),
        ("stamp", json!("v1.2.3")),
        ("url", json!("https://example.com")),
    ] {
        let mut held = address("preview");
        held[field] = value;
        assert!(serde_json::from_value::<Address>(held).is_err(), "{field}");
    }
    for field in ["repository", "app", "lane"] {
        let mut held = address("preview");
        held.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Address>(held).is_err());
    }
    let repeated =
        r#"{"repository":"PerishLab/crest","app":"crest","lane":"preview","lane":"design"}"#;
    assert!(serde_json::from_str::<Address>(repeated).is_err());
}

#[test]
fn separation() {
    let held = serde_json::from_value::<Address>(address("design")).unwrap();
    let sibling = serde_json::from_value::<Address>(address("review")).unwrap();
    assert_ne!(held, sibling);
    let mut target = address("design");
    target["app"] = json!("design");
    assert_ne!(serde_json::from_value::<Address>(target).unwrap(), held);
    let mut target = address("design");
    target["repository"] = json!("PerishLab/design");
    assert_ne!(serde_json::from_value::<Address>(target).unwrap(), held);
}
