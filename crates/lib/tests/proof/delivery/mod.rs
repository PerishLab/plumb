use plumb::delivery::{Narrative, Plan, Reference, SCHEMA, Snapshot};
use serde_json::Value;

mod fixture;
mod settlement;

#[test]
fn surface() {
    let prepare = plumb::delivery::prepare;
    let revalidate = plumb::delivery::revalidate;
    assert_eq!(SCHEMA, "plumb.delivery-plan/v2");
    assert_eq!(std::mem::size_of_val(&prepare), 0);
    assert_eq!(std::mem::size_of_val(&revalidate), 0);

    let fixture = tempfile::tempdir().expect("fixture");
    let path = fixture.path().join("plan.json");
    let plan = plan(fixture.path().to_path_buf());
    std::fs::write(&path, serde_json::to_vec(&plan).expect("plan JSON")).expect("plan file");
    assert_eq!(plumb::delivery::read(&path).expect("read plan"), plan);
}

#[test]
fn wire() {
    let fixture = tempfile::tempdir().expect("fixture");
    let mut plan = plan(fixture.path().to_path_buf());
    let child = Reference {
        number: 38,
        node: "I_child".to_string(),
        url: "https://github.com/PerishLab/plumb/issues/38".to_string(),
        title: "Child".to_string(),
        ..reference()
    };
    let blocker = Reference {
        number: 39,
        node: "I_blocker".to_string(),
        url: "https://github.com/PerishLab/plumb/issues/39".to_string(),
        title: "Blocker".to_string(),
        ..reference()
    };
    plan.issue.children = vec![child];
    plan.issue.blockers = vec![blocker];
    let raw = serde_json::to_string(&plan).expect("plan string");
    let first = raw.find("\"sub_issues\":").expect("sub_issues key");
    let second = raw.find("\"blocked_by\":").expect("blocked_by key");
    assert!(first < second);
    let encoded = serde_json::to_value(&plan).expect("plan value");
    let issue = encoded
        .get("issue")
        .and_then(Value::as_object)
        .expect("issue object");
    assert_eq!(
        issue
            .get("sub_issues")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        issue
            .get("blocked_by")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert!(!issue.contains_key("children"));
    assert!(!issue.contains_key("blockers"));
    let decoded: Plan = serde_json::from_value(encoded.clone()).expect("wire roundtrip");
    assert_eq!(decoded, plan);
    for (from, into) in [("sub_issues", "children"), ("blocked_by", "blockers")] {
        let mut wrong = encoded.clone();
        let fields = wrong["issue"].as_object_mut().expect("mutable issue");
        let moved = fields.remove(from).expect(from);
        fields.insert(into.to_string(), moved);
        assert!(serde_json::from_value::<Plan>(wrong).is_err(), "{into}");
    }
}

#[test]
fn drift() {
    let fixture = tempfile::tempdir().expect("fixture");
    let plan = plan(fixture.path().to_path_buf());
    let mut issue = plan.issue.clone();
    let mut pull = plan.pull.clone();
    assert!(
        plan.agrees(&plumb::delivery::Request {
            root: fixture.path(),
            repository: "PerishLab/plumb",
            issue: &issue,
            observed: 2,
            base: "main",
            pull: &pull,
        })
        .expect("current")
    );
    issue.updated.push_str("-drift");
    assert!(
        !plan
            .agrees(&plumb::delivery::Request {
                root: fixture.path(),
                repository: "PerishLab/plumb",
                issue: &issue,
                observed: 2,
                base: "main",
                pull: &pull,
            })
            .expect("Issue drift")
    );
    issue = plan.issue.clone();
    assert!(
        !plan
            .agrees(&plumb::delivery::Request {
                root: fixture.path(),
                repository: "PerishLab/plumb",
                issue: &issue,
                observed: 2,
                base: "other",
                pull: &pull,
            })
            .expect("base drift")
    );
    pull.body.push_str(" drift");
    assert!(
        !plan
            .agrees(&plumb::delivery::Request {
                root: fixture.path(),
                repository: "PerishLab/plumb",
                issue: &issue,
                observed: 2,
                base: "main",
                pull: &pull,
            })
            .expect("pull drift")
    );
}

fn plan(root: std::path::PathBuf) -> Plan {
    Plan {
        schema: SCHEMA.to_string(),
        root,
        repository: "PerishLab/plumb".to_string(),
        issue: Snapshot {
            node: "I_issue".to_string(),
            repository: "PerishLab/plumb".to_string(),
            number: 37,
            url: "https://github.com/PerishLab/plumb/issues/37".to_string(),
            title: "Narrow delivery".to_string(),
            state: "OPEN".to_string(),
            kind: "Feature".to_string(),
            updated: "2026-09-28T00:00:00Z".to_string(),
            parent: Some(reference()),
            children: Vec::new(),
            blockers: Vec::new(),
            blocking: Vec::new(),
        },
        observed: 1,
        base: "main".to_string(),
        target: "1".repeat(40),
        branch: "topic".to_string(),
        projection: "land/topic".to_string(),
        source: "2".repeat(40),
        candidate: "3".repeat(40),
        pull: Narrative {
            title: "Narrow delivery".to_string(),
            body: "Refs PerishLab/plumb#37".to_string(),
        },
        guard: plumb::landing::Guard {
            schema: "plumb.guard-proof/v1".to_string(),
            tree: "4".repeat(40),
            digest: "5".repeat(64),
        },
    }
}

fn reference() -> Reference {
    Reference {
        repository: "PerishLab/plumb".to_string(),
        number: 24,
        node: "I_parent".to_string(),
        url: "https://github.com/PerishLab/plumb/issues/24".to_string(),
        title: "Parent".to_string(),
        state: "OPEN".to_string(),
    }
}
