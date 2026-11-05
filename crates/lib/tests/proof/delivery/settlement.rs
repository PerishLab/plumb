use super::fixture::{Fixture, HEAD};
use serde_json::json;

#[test]
fn multiple() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.settle().unwrap(),
        "https://github.com/Example/probe/actions/runs/17/job/1"
    );
    assert!(fixture.merged());
    let calls = fixture.calls.borrow();
    let merge = calls.last().unwrap();
    assert_eq!(
        merge,
        &vec![
            "pr",
            "merge",
            "23",
            "-R",
            "Example/probe",
            "--squash",
            "--match-head-commit",
            HEAD,
            "--subject",
            "Qualify matching Guard runs",
            "--body",
            "Refs #484"
        ]
    );
    assert!(!merge.iter().any(|arg| arg == "--admin" || arg == "--auto"));
    for id in [17, 18] {
        assert!(calls.iter().any(|call| call
            == &vec![
                "api".to_string(),
                format!("repos/Example/probe/actions/runs/{id}")
            ]));
    }
}

#[test]
fn paginated() {
    let mut fixture = Fixture::new();
    fixture.checks =
        json!([{"check_runs":[Fixture::check(17)]}, [{"check_runs":[Fixture::check(18)]}]]);
    fixture.workflows["18"]["event"] = json!("merge_group");
    fixture.settle().unwrap();
    assert!(fixture.merged());
}

#[test]
fn single() {
    let mut fixture = Fixture::new();
    fixture.checks = json!([{"check_runs":[Fixture::check(17)]}]);
    fixture.settle().unwrap();
    assert!(fixture.merged());
}

#[test]
fn checks() {
    for index in [0, 1] {
        for patch in [
            json!({"status":"in_progress","conclusion":null}),
            json!({"status":"queued","conclusion":null}),
            json!({"conclusion":"failure"}),
            json!({"conclusion":"cancelled"}),
            json!({"conclusion":null}),
            json!({"conclusion":"skipped"}),
            json!({"details_url":"unreadable"}),
        ] {
            let mut fixture = Fixture::new();
            for (key, value) in patch.as_object().unwrap() {
                fixture.checks[0]["check_runs"][index][key] = value.clone();
            }
            assert!(fixture.settle().is_err(), "{patch}");
            assert!(!fixture.merged());
        }
    }
}

#[test]
fn workflows() {
    for id in ["17", "18"] {
        for patch in [
            json!({"status":"in_progress","conclusion":null}),
            json!({"status":"queued","conclusion":null}),
            json!({"conclusion":"failure"}),
            json!({"conclusion":"cancelled"}),
            json!({"conclusion":null}),
            json!({"head_sha":"foreign"}),
            json!({"path":"other.yml"}),
            json!({"event":"push"}),
        ] {
            let mut fixture = Fixture::new();
            for (key, value) in patch.as_object().unwrap() {
                fixture.workflows[id][key] = value.clone();
            }
            assert!(fixture.settle().is_err(), "{id}: {patch}");
            assert!(!fixture.merged());
        }
    }
}

#[test]
fn unreadable() {
    let mut fixture = Fixture::new();
    fixture.fault = true;
    assert!(fixture.settle().is_err());
    assert!(!fixture.merged());
    for checks in [
        json!([]),
        json!([{"check_runs":[]}]),
        json!([{"check_runs":null}]),
    ] {
        let mut fixture = Fixture::new();
        fixture.checks = checks;
        assert!(fixture.settle().is_err());
        assert!(!fixture.merged());
    }
}

#[test]
fn unrelated() {
    for patch in [
        json!({"head_sha":"foreign"}),
        json!({"name":"other"}),
        json!({"app":{"slug":"foreign"}}),
    ] {
        let mut fixture = Fixture::new();
        let mut check = Fixture::check(19);
        for (key, value) in patch.as_object().unwrap() {
            check[key] = value.clone();
        }
        fixture.checks[0]["check_runs"]
            .as_array_mut()
            .unwrap()
            .push(check);
        fixture.settle().unwrap();
        assert!(fixture.merged());
    }
}
