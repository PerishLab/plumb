use super::{
    material::Codec,
    world::{Change, Fixture, GH, ROOT},
};
use plumb::windtun::qualification::{Inspector, State};
use serde_json::json;

#[test]
fn requirements() {
    let mut fixture = Fixture::new();
    fixture.policy(&"0".repeat(64));
    let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused);
    let mut fixture = Fixture::new();
    let path = format!(
        "{GH}/contents/resources/check/windtun.json?ref={}",
        "a".repeat(40)
    );
    let descriptor = Codec::content(
        "resources/check/windtun.json",
        &Codec::bytes(
            &json!({"schema":"wharf.windtun.requirements/v1","policy":"wharf.windtun.linux/v1","implementation":"0".repeat(64)}),
        ),
    );
    fixture.change = Some(Change {
        path,
        after: 2,
        body: Codec::bytes(&descriptor),
    });
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn discovery() {
    let mut fixture = Fixture::new();
    let path = format!("{ROOT}/latest.json");
    let mut index = fixture.get(&path);
    index["observer"]["attempt"] = json!("2");
    fixture.change = Some(Change {
        path,
        after: 2,
        body: Codec::bytes(&index),
    });
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn world() {
    for path in [
        "https://releases.keel.perish.uk/v1/channels/rc.json",
        "https://releases.seatrial.perish.uk/v1/channels/stable.json",
        "https://depot.seatrial.perish.uk/channels/beta/scenes/latest.json",
    ] {
        let mut fixture = Fixture::new();
        let mut value = fixture.get(path);
        value["commit"] = json!("0".repeat(40));
        fixture.change = Some(Change {
            path: path.into(),
            after: 2,
            body: Codec::bytes(&value),
        });
        assert_eq!(
            Inspector::new(&fixture)
                .inspect("PerishLab/keel", "v1.0.0-rc.1")
                .state,
            State::Refused,
            "{path}"
        );
    }
    let mut fixture = Fixture::new();
    fixture.drift = true;
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn origin() {
    for path in [
        format!("{GH}/branches/main"),
        "https://api.github.com/repos/PerishLab/keel/git/ref/tags/v1.0.0-rc.1".into(),
    ] {
        let mut fixture = Fixture::new();
        let mut value = fixture.get(&path);
        if path.ends_with("branches/main") {
            value["commit"]["sha"] = json!("0".repeat(40));
        } else {
            value["object"]["sha"] = json!("6".repeat(40));
        }
        fixture.change = Some(Change {
            path,
            after: 2,
            body: Codec::bytes(&value),
        });
        assert_eq!(
            Inspector::new(&fixture)
                .inspect("PerishLab/keel", "v1.0.0-rc.1")
                .state,
            State::Refused
        );
    }
}

#[test]
fn format() {
    for (field, value) in [
        ("schema", json!("unknown")),
        ("policy", json!("foreign")),
        ("implementation", json!(true)),
        ("extra", json!("foreign")),
    ] {
        let mut fixture = Fixture::new();
        let mut descriptor = json!({"schema":"wharf.windtun.requirements/v1","policy":"wharf.windtun.linux/v1","implementation":"c".repeat(64)});
        descriptor[field] = value;
        fixture.set(
            &format!(
                "{GH}/contents/resources/check/windtun.json?ref={}",
                "a".repeat(40)
            ),
            &Codec::content("resources/check/windtun.json", &Codec::bytes(&descriptor)),
        );
        assert_eq!(
            Inspector::new(&fixture)
                .inspect("PerishLab/keel", "v1.0.0-rc.1")
                .state,
            State::Refused,
            "{field}"
        );
    }
}

#[test]
fn late() {
    use plumb::windtun::qualification::{Read, Result};
    use std::cell::Cell;
    struct Pending<'a> {
        fixture: &'a Fixture,
        reads: Cell<usize>,
    }
    impl Read for Pending<'_> {
        fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
            let raw = self.fixture.bytes(url, limit)?;
            if url.contains("/contents/resources/check/windtun.json") {
                self.reads.set(self.reads.get() + 1);
            }
            if url == format!("{ROOT}/latest.json") && self.reads.get() >= 2 {
                let mut index = self.fixture.get(url);
                index["observer"]["attempt"] = json!("2");
                return Ok(Codec::bytes(&index));
            }
            Ok(raw)
        }
        fn image(&self, reference: &str) -> Result<String> {
            self.fixture.image(reference)
        }
    }
    let fixture = Fixture::new();
    let report = Inspector::new(Pending {
        fixture: &fixture,
        reads: Cell::new(0),
    })
    .inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused);
}
