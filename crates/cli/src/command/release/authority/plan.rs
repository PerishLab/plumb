use serde::Serialize;

use super::{Action, Model, Observation};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Ready,
    Change,
    Deferred,
}

#[derive(Debug, Serialize)]
pub struct Step {
    pub resource: &'static str,
    pub status: Status,
    pub detail: String,
}

pub fn build(model: &Model, seen: &Observation) -> (Vec<Step>, Option<Action>) {
    if model.profile == "ship" {
        return ship(model, seen);
    }
    let mut plan = Builder::default();
    if seen.bucket {
        plan.ready(
            resource(model, "bucket"),
            format!("{} exists", model.bucket),
        );
    } else {
        plan.change(
            resource(model, "bucket"),
            format!("create R2 bucket {}", model.bucket),
            Action::Bucket,
        );
    }
    if !seen.bucket {
        plan.deferred(resource(model, "domain"), wait(model, "bucket"));
    } else if seen.domain.as_ref().is_some_and(|held| held.ready()) {
        plan.ready(
            resource(model, "domain"),
            format!("{} serves active TLS 1.2", model.domain),
        );
    } else {
        plan.change(
            resource(model, "domain"),
            format!(
                "attach or normalize {} in zone {}",
                model.domain, model.zone
            ),
            Action::Domain,
        );
    }
    if !seen.bucket || !seen.domain.as_ref().is_some_and(|held| held.ready()) {
        plan.deferred(resource(model, "capability"), wait(model, "domain"));
    } else if seen.recovery {
        plan.change(
            resource(model, "capability"),
            "recover the bucket-scoped writer and its local escrow",
            Action::Recovery,
        );
    } else if seen.capability.is_some() && seen.escrow.is_some() {
        plan.ready(
            resource(model, "capability"),
            "bucket-scoped writer matches its local escrow",
        );
    } else {
        plan.change(
            resource(model, "capability"),
            format!("mint {} and retain its one-time secret", model.writer()),
            Action::Capability,
        );
    }
    if seen.escrow.is_none() {
        plan.deferred(resource(model, "secrets"), wait(model, "capability"));
    } else if model
        .secrets()
        .iter()
        .all(|name| seen.secrets.contains(*name))
    {
        plan.ready(
            resource(model, "secrets"),
            match model.profile {
                "release" => "all five opaque release publish seats are present",
                "workflow" => "all five opaque workflow inventory seats are present",
                _ => "all authority secret seats are present",
            },
        );
    } else {
        plan.change(
            resource(model, "secrets"),
            match model.profile {
                "release" => "upsert exactly the five release publish secrets",
                "workflow" => "upsert exactly the five workflow inventory secrets",
                _ => "converge the authority secrets",
            },
            Action::Repository,
        );
    }
    plan.finish()
}

fn ship(target: &Model, seen: &Observation) -> (Vec<Step>, Option<Action>) {
    let mut plan = Builder::default();
    if seen.recovery {
        plan.change(
            "ship.capability",
            "recover the all-release-buckets writer and its local escrow",
            Action::Recovery,
        );
    } else if seen.capability.is_some() && seen.escrow.is_some() {
        plan.ready(
            "ship.capability",
            "all-release-buckets writer matches its local escrow",
        );
    } else {
        plan.change(
            "ship.capability",
            format!("mint {} and retain its one-time secret", target.writer()),
            Action::Capability,
        );
    }
    if seen.escrow.is_none() {
        plan.deferred("repository.secrets", "waiting for ship.capability");
    } else if target
        .secrets()
        .iter()
        .all(|name| seen.secrets.contains(*name))
    {
        plan.ready(
            "repository.secrets",
            "all four opaque central ship publish seats are present",
        );
    } else {
        plan.change(
            "repository.secrets",
            "upsert exactly the four central ship publish secrets",
            Action::Repository,
        );
    }
    plan.finish()
}

fn resource(model: &Model, name: &'static str) -> &'static str {
    match (model.profile, name) {
        ("release", "bucket") => "release.bucket",
        ("release", "domain") => "release.domain",
        ("release", "capability") => "release.capability",
        ("release", "secrets") => "repository.secrets",
        ("workflow", "bucket") => "workflow.bucket",
        ("workflow", "domain") => "workflow.domain",
        ("workflow", "capability") => "workflow.capability",
        ("workflow", "secrets") if model.organization().is_some() => "organization.secrets",
        ("workflow", "secrets") => "repository.secrets",
        _ => "authority.resource",
    }
}

fn wait(model: &Model, name: &str) -> String {
    format!("waiting for {}.{name}", model.profile)
}

#[derive(Default)]
struct Builder {
    steps: Vec<Step>,
    action: Option<Action>,
}

impl Builder {
    fn ready(&mut self, resource: &'static str, detail: impl Into<String>) {
        self.push(resource, Status::Ready, detail);
    }

    fn deferred(&mut self, resource: &'static str, detail: impl Into<String>) {
        self.push(resource, Status::Deferred, detail);
    }

    fn change(&mut self, resource: &'static str, detail: impl Into<String>, action: Action) {
        self.push(resource, Status::Change, detail);
        self.action.get_or_insert(action);
    }

    fn push(&mut self, resource: &'static str, status: Status, detail: impl Into<String>) {
        self.steps.push(Step {
            resource,
            status,
            detail: detail.into(),
        });
    }

    fn finish(self) -> (Vec<Step>, Option<Action>) {
        (self.steps, self.action)
    }
}
