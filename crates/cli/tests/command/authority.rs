use serde::Serialize;
use std::collections::BTreeSet;
use std::process::Command;

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
    organization: bool,
}

impl Model {
    fn writer(&self) -> String {
        format!("publish:{}", self.bucket)
    }

    fn secrets(&self) -> &'static [&'static str] {
        self.secrets
    }

    fn organization(&self) -> Option<&str> {
        self.organization.then_some("PerishLab")
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
        organization: false,
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
        organization: false,
    };
    assert_eq!(model.writer(), "publish:perish-probe-releases");
}

#[test]
fn workflow() {
    const HELD: [&str; 5] = ["access", "secret", "bucket", "endpoint", "url"];
    let mut model = Model {
        profile: "workflow",
        secrets: &HELD,
        bucket: "perish-workflow-inventory".into(),
        domain: "inventory.plumb.test".into(),
        zone: "zone".into(),
        organization: true,
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
    model.organization = false;
    let (steps, _) = plan::build(&model, &seen);
    assert_eq!(steps[3].resource, "repository.secrets");
    let root = std::env::temp_dir().join("plumb-workflow-organization");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("old fixture should be swept");
    }
    std::fs::create_dir_all(&root).expect("fixture should be made");
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("git should run")
            .success()
    );
    assert!(
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "ssh://git@git.perish.top/PerishLab/actions.git"
            ])
            .current_dir(&root)
            .status()
            .expect("git should run")
            .success()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "authority",
            "workflow",
            root.to_str().expect("fixture path should be utf8"),
            "--domain",
            "inventory.plumb.test",
            "--zone-id",
            "zone",
            "--organization",
            "PerishFire/escape",
        ])
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--organization must name one Forgejo organization")
    );
}
