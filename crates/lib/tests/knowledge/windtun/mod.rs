mod binary;
mod coherent;
mod current;
mod material;
mod receipt;
mod refused;
mod world;

use plumb::windtun::qualification::{Inspector, State};
use world::Fixture;

#[test]
fn qualification() {
    let fixture = Fixture::new();
    let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Qualified, "{report:?}");
    assert!(report.qualified);
    assert_eq!(report.association, None);
    assert!(!report.promotion);
    assert_eq!(report.platform, "linux-x86_64");
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["evidence"]["requirements"]["commit"], "a".repeat(40));
    assert_eq!(value["evidence"]["observer"]["wharf"], "d".repeat(40));
}

#[test]
fn selectors() {
    let fixture = Fixture::new();
    for (repository, marker) in [
        ("fork/keel", "v1.0.0-rc.1"),
        ("PerishLab/../keel", "v1.0.0-rc.1"),
        ("PerishLab/keel", "v1.0.0"),
        ("PerishLab/keel", "v1.0.0-rc.0"),
        ("PerishLab/keel", "v1.0.0-rc.1+foreign"),
    ] {
        let report = Inspector::new(&fixture).inspect(repository, marker);
        assert_eq!(
            report.state,
            State::Refused,
            "{repository}/{marker}: {report:?}"
        );
        assert!(!report.promotion);
    }
    assert!(fixture.counts.borrow().is_empty());
}
