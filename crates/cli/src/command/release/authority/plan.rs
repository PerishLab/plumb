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
    let mut plan = Builder::default();
    if seen.bucket {
        plan.ready("release.bucket", format!("{} exists", model.bucket));
    } else {
        plan.change(
            "release.bucket",
            format!("create R2 bucket {}", model.bucket),
            Action::Bucket,
        );
    }
    if !seen.bucket {
        plan.deferred("release.domain", "waiting for release.bucket");
    } else if seen.domain.as_ref().is_some_and(|held| held.ready()) {
        plan.ready(
            "release.domain",
            format!("{} serves active TLS 1.2", model.domain),
        );
    } else {
        plan.change(
            "release.domain",
            format!(
                "attach or normalize {} in zone {}",
                model.domain, model.zone
            ),
            Action::Domain,
        );
    }
    if !seen.bucket || !seen.domain.as_ref().is_some_and(|held| held.ready()) {
        plan.deferred("release.capability", "waiting for release.domain");
    } else if seen.capability.is_some() && seen.escrow.is_some() {
        plan.ready(
            "release.capability",
            "bucket-scoped writer matches its local escrow",
        );
    } else {
        plan.change(
            "release.capability",
            format!("mint {} and retain its one-time secret", model.writer()),
            Action::Capability,
        );
    }
    if seen.escrow.is_none() {
        plan.deferred("repository.secrets", "waiting for release.capability");
    } else if super::SECRETS
        .iter()
        .all(|name| seen.secrets.contains(*name))
    {
        plan.ready(
            "repository.secrets",
            "all four opaque publish seats are present",
        );
    } else {
        plan.change(
            "repository.secrets",
            "upsert exactly the four release publish secrets",
            Action::Repository,
        );
    }
    plan.finish()
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
