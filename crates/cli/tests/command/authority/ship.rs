use super::super::cloud;
use super::*;

#[test]
fn ship() {
    const HELD: [&str; 4] = ["access", "secret", "endpoint", "fingerprint"];
    let model = Model {
        profile: "ship",
        secrets: &HELD,
        bucket: "perish-plumb-releases".into(),
        domain: String::new(),
        zone: String::new(),
        organization: false,
    };
    let mut seen = Observation {
        bucket: true,
        domain: None,
        capability: None,
        recovery: false,
        escrow: None,
        secrets: BTreeSet::new(),
        policy: None,
    };
    let (steps, action) = plan::build(&model, &seen);
    assert_eq!(action, Some(Action::Capability));
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].resource, "ship.capability");
    assert_eq!(steps[1].resource, "repository.secrets");

    seen.capability = Some("writer".into());
    seen.escrow = Some(());
    seen.secrets.extend(HELD.map(str::to_string));
    let (steps, action) = plan::build(&model, &seen);
    assert_eq!(action, None);
    assert_eq!(steps[0].status, plan::Status::Ready);
    assert_eq!(steps[1].status, plan::Status::Ready);
    seen.policy = Some(
        cloud::adapter::Policy::read(
            serde_json::json!({"status":"active", "policies":[{
                "effect":"allow", "resources":{}, "permission_groups":[{"id":"permit"}]
            }]}),
            serde_json::json!({"concord":"*"}),
            "permit",
        )
        .unwrap(),
    );
    let (steps, action) = plan::build(&model, &seen);
    assert_eq!(action, Some(Action::Policy));
    assert!(steps[0].detail.contains("without rotating"));
}
