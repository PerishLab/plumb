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
    bucket: String,
    domain: String,
    zone: String,
}

impl Model {
    fn writer(&self) -> String {
        format!("publish:{}", self.bucket)
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
        bucket: "perish-probe-releases".into(),
        domain: "releases.probe.test".into(),
        zone: "zone".into(),
    };
    assert_eq!(model.writer(), "publish:perish-probe-releases");
}
