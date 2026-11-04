use super::{material::Codec, world::Fixture};
use plumb::windtun::qualification::{Inspector, Report, State};
use serde_json::{Value, json};

const URL: &str = "https://releases.keel.perish.uk/v1/releases/rc/v1.0.0-rc.1/distribution.json";

const KINDS: &[&str] = &[
    "schema",
    "upper",
    "commit",
    "identity",
    "zero",
    "wide",
    "number",
    "missing",
    "extra",
    "empty",
    "array",
    "null",
    "repository",
    "marker",
    "foreign",
    "tree",
    "run",
    "wharf",
    "future",
    "lexical",
    "overflow",
    "huge",
    "digest",
    "short",
    "caps",
    "added",
    "dropped",
    "legacy",
    "crossed",
    "between",
];

fn load(fixture: &Fixture) -> Value {
    fixture.get(URL)
}

fn store(fixture: &mut Fixture, distribution: &Value) {
    fixture.set(URL, distribution);
    fixture.pass["distribution"] = json!(Codec::digest(distribution));
    fixture.pass["publisher"] = distribution["completed"].clone();
    fixture.publish();
}

fn reference(distribution: &Value, attempt: &str) -> Value {
    json!({
        "schema": "wharf.plan.reference/v1",
        "repository": distribution["repository"],
        "marker": distribution["marker"],
        "commit": distribution["commit"],
        "tree": distribution["tree"],
        "wharf": distribution["completed"]["wharf"],
        "run": distribution["completed"]["run"],
        "attempt": attempt,
        "sha256": "a".repeat(64),
    })
}

fn prepare(fixture: &mut Fixture, completed: &str, attempt: &str, later: bool) {
    let mut distribution = load(fixture);
    distribution["completed"]["attempt"] = json!(completed);
    distribution["attempt"] = distribution["completed"].clone();
    distribution["plan"] = reference(&distribution, attempt);
    if later {
        distribution["attempt"] = json!({
            "run": "11",
            "attempt": "1",
            "wharf": "c".repeat(40),
        });
    }
    store(fixture, &distribution);
}

fn inspect(fixture: &Fixture) -> Report {
    Inspector::new(fixture).inspect("PerishLab/keel", "v1.0.0-rc.1")
}

fn accept(report: &Report) {
    assert_eq!(report.state, State::Qualified, "{report:?}");
    assert!(report.qualified);
    assert!(report.evidence.is_some());
    assert!(report.reason.is_none());
    assert_eq!(report.association, None);
    assert!(!report.promotion);
}

fn reject(report: &Report, kind: &str) {
    assert_eq!(report.state, State::Refused, "{kind}: {report:?}");
    assert!(!report.qualified);
    assert!(report.evidence.is_none());
    assert!(report.reason.is_some());
    assert_eq!(report.association, None);
    assert!(!report.promotion);
}

fn spoil(distribution: &mut Value, kind: &str) {
    match kind {
        "schema" => distribution["plan"]["schema"] = json!("wharf.plan.reference/v2"),
        "upper" => distribution["plan"]["commit"] = json!("A".repeat(40)),
        "commit" => distribution["plan"]["commit"] = json!("zz"),
        "identity" => distribution["plan"]["run"] = json!("01"),
        "zero" => distribution["plan"]["attempt"] = json!("0"),
        "wide" => distribution["plan"]["run"] = json!("1".repeat(21)),
        "number" => distribution["plan"]["attempt"] = json!(1),
        "missing" => {
            distribution["plan"].as_object_mut().unwrap().remove("tree");
        }
        "extra" => distribution["plan"]["note"] = json!(true),
        "empty" => distribution["plan"] = json!({}),
        "array" => distribution["plan"] = json!([]),
        "null" => distribution["plan"] = Value::Null,
        "repository" => distribution["plan"]["repository"] = json!("PerishLab/other"),
        "marker" => distribution["plan"]["marker"] = json!("v9.9.9-rc.9"),
        "foreign" => distribution["plan"]["commit"] = json!("9".repeat(40)),
        "tree" => distribution["plan"]["tree"] = json!("9".repeat(40)),
        "run" => distribution["plan"]["run"] = json!("8"),
        "wharf" => distribution["plan"]["wharf"] = json!("f".repeat(40)),
        "future" => distribution["plan"]["attempt"] = json!("2"),
        "lexical" => distribution["plan"]["attempt"] = json!("10"),
        "overflow" => distribution["plan"]["attempt"] = json!("18446744073709551616"),
        "huge" => distribution["plan"]["run"] = json!("9".repeat(20)),
        "digest" => distribution["plan"]["sha256"] = json!("g".repeat(64)),
        "short" => distribution["plan"]["sha256"] = json!("a".repeat(63)),
        "caps" => distribution["plan"]["sha256"] = json!("A".repeat(64)),
        "added" => distribution["note"] = json!(true),
        "dropped" => {
            distribution.as_object_mut().unwrap().remove("trial");
        }
        "legacy" => {
            distribution.as_object_mut().unwrap().remove("plan");
            distribution["note"] = json!(true);
        }
        "crossed" => {
            distribution["attempt"] = json!({"run":"11","attempt":"1","wharf":"c".repeat(40)});
            distribution["plan"]["run"] = json!("11");
            distribution["plan"]["wharf"] = json!("c".repeat(40));
            distribution["plan"]["attempt"] = json!("1");
        }
        "between" => {
            distribution["attempt"] = json!({"run":"9","attempt":"5","wharf":"e".repeat(40)});
            distribution["plan"]["attempt"] = json!("3");
        }
        _ => panic!("unknown {kind}"),
    }
}

#[test]
fn legacy() {
    accept(&inspect(&Fixture::new()));
}

#[test]
fn admitted() {
    for (completed, attempt, later) in [
        ("1", "1", false),
        ("2", "1", false),
        ("10", "9", false),
        ("18446744073709551615", "18446744073709551615", false),
        ("1", "1", true),
        ("2", "1", true),
        ("2", "2", true),
    ] {
        let mut fixture = Fixture::new();
        prepare(&mut fixture, completed, attempt, later);
        accept(&inspect(&fixture));
    }
    let mut fixture = Fixture::new();
    prepare(&mut fixture, "2", "2", true);
    fixture.pass["distribution"] = json!("b".repeat(64));
    fixture.publish();
    reject(&inspect(&fixture), "bound");
}

#[test]
fn refused() {
    for kind in KINDS {
        let mut fixture = Fixture::new();
        let completed = if *kind == "lexical" { "9" } else { "1" };
        prepare(&mut fixture, completed, "1", false);
        let mut distribution = load(&fixture);
        spoil(&mut distribution, kind);
        store(&mut fixture, &distribution);
        let report = inspect(&fixture);
        reject(&report, kind);
        let expected = match *kind {
            "identity" | "zero" | "wide" => Some("invalid plan identity"),
            "huge" => Some("plan reference run differs"),
            "overflow" => Some("plan reference attempt follows completion"),
            _ => None,
        };
        if let Some(expected) = expected {
            assert_eq!(report.reason.as_deref(), Some(expected), "{kind}");
        }
    }
}
