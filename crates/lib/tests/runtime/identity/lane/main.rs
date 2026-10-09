use plumb::lane::{Address, Name};
use serde_json::{Value, json};

fn address(lane: &str) -> Value {
    json!({"repository":"PerishLab/crest","app":"crest","lane":lane})
}

#[test]
fn names() {
    for (text, family, key) in [
        ("preview", "preview", None),
        ("preview.crest-a", "preview", Some("crest-a")),
        ("preview.a-10", "preview", Some("a-10")),
        ("lane.experiment", "lane", Some("experiment")),
    ] {
        let name = Name::read(text).unwrap();
        assert_eq!(name.value(), text);
        assert_eq!(name.family(), family);
        assert_eq!(name.key(), key);
        assert_eq!(serde_json::to_value(&name).unwrap(), json!(text));
        assert_eq!(serde_json::from_value::<Name>(json!(text)).unwrap(), name);
    }
}

#[test]
fn refusals() {
    for text in [
        "",
        "Preview",
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
        "preview.a\n",
        " preview",
        "v1.2.3-lane.a",
    ] {
        assert!(Name::read(text).is_err(), "{text:?}");
        assert!(serde_json::from_value::<Name>(json!(text)).is_err());
    }
    for text in ["a".repeat(49), format!("preview.{}", "a".repeat(49))] {
        assert!(Name::read(&text).is_err());
    }
    let text = format!("{}.{}", "a".repeat(48), "b".repeat(48));
    assert!(Name::read(&text).is_ok());
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
fn targets() {
    let default = serde_json::from_value::<Address>(address("preview")).unwrap();
    assert_eq!(default.repository(), "PerishLab/crest");
    assert_eq!(default.app(), "crest");
    assert_eq!(default.lane().key(), None);
    assert_eq!(serde_json::to_value(&default).unwrap(), address("preview"));
    for lane in ["preview.a", "preview.b"] {
        let target = serde_json::from_value::<Address>(address(lane)).unwrap();
        assert_ne!(target, default);
        assert_eq!(target.app(), default.app());
    }
    let name = Name::read("preview").unwrap();
    assert_eq!(
        Address::new("PerishLab/crest".into(), "crest".into(), name).unwrap(),
        default
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
        ("lane", json!("preview.a.b")),
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
        r#"{"repository":"PerishLab/crest","app":"crest","lane":"preview","lane":"preview.a"}"#;
    assert!(serde_json::from_str::<Address>(repeated).is_err());
}

#[test]
fn separation() {
    let dotted = serde_json::from_value::<Address>(address("preview.a")).unwrap();
    let hyphen = serde_json::from_value::<Address>(address("preview-a")).unwrap();
    assert_ne!(dotted, hyphen);
    let mut sibling = address("preview.a");
    sibling["app"] = json!("design");
    assert_ne!(serde_json::from_value::<Address>(sibling).unwrap(), dotted);
    let mut other = address("preview.a");
    other["repository"] = json!("PerishLab/design");
    assert_ne!(serde_json::from_value::<Address>(other).unwrap(), dotted);
}
