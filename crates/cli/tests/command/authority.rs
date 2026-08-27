use serde::Serialize;
use std::collections::BTreeSet;

const SECRETS: [&str; 4] = ["access", "secret", "bucket", "endpoint"];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Bucket,
    Domain,
    Capability,
    Repository,
}

struct Model {
    profile: &'static str,
    secrets: &'static [&'static str],
    bucket: String,
    domain: String,
    zone: String,
}

impl Model {
    fn writer(&self) -> String {
        format!("publish:{}", self.bucket)
    }

    fn secrets(&self) -> &'static [&'static str] {
        self.secrets
    }
}

struct Custom(bool);

impl Custom {
    fn ready(&self) -> bool {
        self.0
    }
}

struct Observation {
    bucket: bool,
    domain: Option<Custom>,
    capability: Option<String>,
    escrow: Option<()>,
    secrets: BTreeSet<String>,
}

#[path = "../../src/command/release/authority/plan.rs"]
mod plan;

#[test]
fn ordered() {
    let model = Model {
        profile: "release",
        secrets: &SECRETS,
        bucket: "perish-probe-releases".into(),
        domain: "releases.probe.test".into(),
        zone: "zone".into(),
    };
    let mut seen = Observation {
        bucket: false,
        domain: None,
        capability: None,
        escrow: None,
        secrets: BTreeSet::new(),
    };
    assert_eq!(plan::build(&model, &seen).1, Some(Action::Bucket));

    seen.bucket = true;
    assert_eq!(plan::build(&model, &seen).1, Some(Action::Domain));

    seen.domain = Some(Custom(true));
    assert_eq!(plan::build(&model, &seen).1, Some(Action::Capability));

    seen.capability = Some("writer".into());
    seen.escrow = Some(());
    assert_eq!(plan::build(&model, &seen).1, Some(Action::Repository));

    seen.secrets.extend(SECRETS.map(str::to_string));
    assert_eq!(plan::build(&model, &seen).1, None);
}

#[test]
fn identity() {
    let model = Model {
        profile: "release",
        secrets: &SECRETS,
        bucket: "perish-probe-releases".into(),
        domain: "releases.probe.test".into(),
        zone: "zone".into(),
    };
    assert_eq!(model.writer(), "publish:perish-probe-releases");
}

#[test]
fn workflow() {
    const HELD: [&str; 5] = ["access", "secret", "bucket", "endpoint", "url"];
    let model = Model {
        profile: "workflow",
        secrets: &HELD,
        bucket: "perish-workflow-inventory".into(),
        domain: "inventory.plumb.test".into(),
        zone: "zone".into(),
    };
    let mut seen = Observation {
        bucket: true,
        domain: Some(Custom(true)),
        capability: Some("writer".into()),
        escrow: None,
        secrets: SECRETS.map(str::to_string).into_iter().collect(),
    };
    let (steps, _) = plan::build(&model, &seen);
    assert_eq!(steps[3].resource, "organization.secrets");
    assert_eq!(steps[3].detail, "waiting for workflow.capability");
    seen.escrow = Some(());
    let (steps, action) = plan::build(&model, &seen);
    assert_eq!(action, Some(Action::Repository));
    assert_eq!(steps[3].resource, "organization.secrets");
    assert_eq!(
        steps[3].detail,
        "upsert exactly the five workflow inventory secrets"
    );
    seen.secrets.insert("url".into());
    let (steps, action) = plan::build(&model, &seen);
    assert_eq!(action, None);
    assert_eq!(steps[3].resource, "organization.secrets");
    assert_eq!(
        steps[3].detail,
        "all five opaque workflow inventory seats are present"
    );
}
