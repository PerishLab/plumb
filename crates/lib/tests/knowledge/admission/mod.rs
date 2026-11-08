mod baseline;
mod comparison;
mod fixture;
mod inputs;
mod producer;
mod runtime;

use fixture::{COMMENT, Fixture, LOCATION, PERMISSION, WHARF};
use plumb::semantics::admission::{Inspector, Judgment, Requirements};
use serde_json::json;

#[test]
fn authority() {
    let observed = Inspector::new(Fixture::new()).current().unwrap();
    assert_eq!(observed.source.repository, "PerishLab/wharf");
    assert_eq!(observed.source.commit, "a".repeat(40));
    assert_eq!(observed.source.tree, "b".repeat(40));
    assert!(observed.requirements.scopes.is_empty());
    assert_eq!(observed.path, "resources/check/semantics.json");
}

#[test]
fn deployment() {
    for (url, field, value) in [
        (WHARF.to_string(), "id", json!(1)),
        (WHARF.to_string(), "node_id", json!("foreign")),
        (
            WHARF.to_string(),
            "owner",
            json!({"id":1,"node_id":"O_foreign","login":"PerishLab"}),
        ),
        (format!("{WHARF}/branches/main"), "protected", json!(false)),
        (format!("{WHARF}/branches/main"), "name", json!("feature")),
        (
            format!("{WHARF}/git/commits/{}", "a".repeat(40)),
            "sha",
            json!("c".repeat(40)),
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit(&url, |held| held[field] = value);
        assert!(Inspector::new(fixture).current().is_err(), "{url}/{field}");
    }
    let mut fixture = Fixture::new();
    fixture.moved = true;
    assert!(Inspector::new(fixture).current().is_err());
}

#[test]
fn blob() {
    let url = format!(
        "{WHARF}/contents/resources/check/semantics.json?ref={}",
        "a".repeat(40)
    );
    for (field, value) in [
        ("path", json!("other.json")),
        ("type", json!("symlink")),
        ("size", json!(true)),
        ("size", json!(999999)),
        ("sha", json!("0".repeat(40))),
        ("content", json!("e30=")),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit(&url, |held| held[field] = value);
        assert!(Inspector::new(fixture).current().is_err(), "{field}");
    }
    let mut fixture = Fixture::new();
    fixture.values.remove(&url);
    assert!(Inspector::new(fixture).current().is_err());
}

#[test]
fn declarations() {
    for raw in [
        "{}",
        "{\"schema\":\"wharf.release.semantics.requirements/v1\",\"scopes\":[],\"scopes\":[]}",
        "{\"schema\":\"wharf.release.semantics.requirements/v1\",\"scopes\":[],\"enabled\":false}",
        "{\"schema\":\"foreign\",\"scopes\":[]}",
        "{\"schema\":\"wharf.release.semantics.requirements/v1\",\"scopes\":[{\"product\":\"plumb\"}]}",
    ] {
        assert!(Requirements::parse(raw.as_bytes()).is_err());
    }
}

#[test]
fn review() {
    let observed = Inspector::new(Fixture::new())
        .review(LOCATION, "plumb")
        .unwrap();
    assert_eq!(observed.author.id, 23);
    assert_eq!(observed.author.login, "alice");
    assert_eq!(observed.author.permission, "write");
    assert_eq!(observed.judgment.before, "a".repeat(64));
    assert_eq!(observed.reference.url, LOCATION);
    assert_eq!(observed.comment, "IC_native");
}

#[test]
fn permission() {
    for (field, value) in [
        ("permission", json!("read")),
        ("user", json!({"id":24,"login":"alice","type":"User"})),
        ("user", json!({"id":23,"login":"other","type":"User"})),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit(PERMISSION, |held| held[field] = value);
        assert!(Inspector::new(fixture).review(LOCATION, "plumb").is_err());
    }
    let mut fixture = Fixture::new();
    fixture.values.remove(PERMISSION);
    assert!(Inspector::new(fixture).review(LOCATION, "plumb").is_err());
}

#[test]
fn identity() {
    for (field, value) in [
        ("id", json!(8)),
        (
            "html_url",
            json!("https://github.com/foreign/repo/issues/178#issuecomment-7"),
        ),
        (
            "issue_url",
            json!("https://api.github.com/repos/PerishLab/.github/issues/179"),
        ),
        ("user", json!({"id":23,"login":"alice","type":"Bot"})),
    ] {
        let mut fixture = Fixture::new();
        fixture.edit(COMMENT, |held| held[field] = value);
        assert!(Inspector::new(fixture).review(LOCATION, "plumb").is_err());
    }
    assert!(
        Inspector::new(Fixture::new())
            .review(
                "https://github.com/PerishLab/.github/issues/178#issuecomment-07",
                "plumb"
            )
            .is_err()
    );
    let mut fixture = Fixture::new();
    fixture.edited = true;
    assert!(Inspector::new(fixture).review(LOCATION, "plumb").is_err());
}

#[test]
fn judgments() {
    let raw = json!({"schema":Judgment::SCHEMA,"product":"plumb","before":"a".repeat(64),"after":"b".repeat(64),"anchor":"c".repeat(64),"decision":"same","basis":"actual bound review"});
    for (field, replacement) in [
        ("decision", json!("major")),
        ("basis", json!(" ")),
        ("before", json!("wrong")),
        ("authorized", json!(true)),
    ] {
        let mut value = raw.clone();
        value[field] = replacement;
        let body = format!("<!-- plumb.release.review/v1\n{value}\n-->");
        assert!(Judgment::parse(&body).is_err());
    }
}

#[test]
fn adoption() {
    let value = json!({"schema":"plumb.release.adoption/v1","product":"plumb",
        "marker":"v0.138.0","snapshot":"a".repeat(64),"world":"b".repeat(64),
        "basis":"Original formal publication materials; no claim of historical compliance"});
    let mut fixture = Fixture::new();
    fixture.edit(COMMENT, |held| {
        held["body"] = format!("<!-- plumb.release.adoption/v1\n{value}\n-->").into()
    });
    let observed = Inspector::new(fixture).adoption(LOCATION, "plumb").unwrap();
    assert_eq!(observed.judgment.marker, "v0.138.0");
    assert_eq!(observed.author.id, 23);
    for (field, replacement) in [
        ("marker", json!("v0.138.0-rc.1")),
        ("basis", json!(" ")),
        ("world", json!("missing")),
        ("authorized", json!(true)),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        let body = format!("<!-- plumb.release.adoption/v1\n{changed}\n-->");
        assert!(plumb::semantics::admission::Definition::parse(&body).is_err());
    }
}
