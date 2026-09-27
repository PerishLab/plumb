use plumb::delivery::{Narrative, Plan, Reference, SCHEMA, Snapshot};

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
            sub_issues: Vec::new(),
            blocked_by: Vec::new(),
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
