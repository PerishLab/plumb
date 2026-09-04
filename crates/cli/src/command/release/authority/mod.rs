mod action;
pub(in crate::command) mod cloudflare;
mod context;
mod escrow;
mod model;
mod plan;
mod registry;

use clap::Subcommand;
use context::Context;
use model::{Model, Release, Workflow};
use serde::Serialize;
use std::collections::BTreeSet;

use cloudflare::Custom;

const ADMIN: (&str, &str) = ("Workers R2 Storage Write", "com.cloudflare.api.account");
const ITEM: (&str, &str) = (
    "Workers R2 Storage Bucket Item Write",
    "com.cloudflare.edge.r2.bucket",
);
const SECRETS: [&str; 4] = [
    "RELEASE_PUBLISH_S3_ACCESS_KEY",
    "RELEASE_PUBLISH_S3_SECRET_KEY",
    "RELEASE_PUBLISH_S3_BUCKET",
    "RELEASE_PUBLISH_S3_ENDPOINT",
];
fn product(root: &std::path::Path) -> Result<String, String> {
    crate::shape::release::Spec::controller(root).map(|spec| spec.product)
}

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Converge the closed release-delivery authority profile")]
    Release(Release),
    #[command(about = "Converge the closed shared-workflow inventory authority profile")]
    Workflow(Workflow),
    #[command(about = "Converge the closed package-registry authority profile")]
    Registry(registry::Input),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Bucket,
    Domain,
    Capability,
    Recovery,
    Repository,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    bucket: bool,
    domain: Option<Custom>,
    capability: Option<String>,
    recovery: bool,
    escrow: Option<escrow::View>,
    secrets: BTreeSet<String>,
}

struct Plan {
    context: Context,
    seen: Observation,
    steps: Vec<plan::Step>,
    action: Option<Action>,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: String,
    profile: &'a str,
    product: &'a str,
    bucket: &'a str,
    domain: &'a str,
    fingerprint: String,
    state: &'static str,
    steps: &'a [plan::Step],
    next: Option<Action>,
}

pub fn run(deed: Deed) -> i32 {
    let result = match deed {
        Deed::Release(release) => execute(release),
        Deed::Workflow(input) => workflow(input),
        Deed::Registry(registry) => registry::execute(registry),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("plumb authority: {error}");
            1
        }
    }
}

fn execute(input: Release) -> Result<(), String> {
    let apply = input.apply;
    let json = input.json;
    let model = Model::read(input)?;
    let mut held = Plan::inspect(model)?;
    if !apply {
        return held.print(json);
    }
    loop {
        if held.action.is_none() {
            return held.print(json);
        }
        held.apply()?;
        held = Plan::inspect(held.context.model.clone())?;
    }
}

fn workflow(input: Workflow) -> Result<(), String> {
    let apply = input.apply;
    let json = input.json;
    converge(Model::workflow(input)?, apply, json)
}

fn converge(model: Model, apply: bool, json: bool) -> Result<(), String> {
    let mut held = Plan::inspect(model)?;
    if !apply {
        return held.print(json);
    }
    loop {
        if held.action.is_none() {
            return held.print(json);
        }
        held.apply()?;
        held = Plan::inspect(held.context.model.clone())?;
    }
}

impl Plan {
    fn inspect(model: Model) -> Result<Self, String> {
        let context = Context::open(model)?;
        let seen = context.observe()?;
        let (steps, action) = plan::build(&context.model, &seen);
        Ok(Self {
            context,
            seen,
            steps,
            action,
        })
    }

    fn print(&self, json: bool) -> Result<(), String> {
        let report = Report {
            schema: format!("plumb.{}-authority-plan/v1", self.context.model.profile),
            profile: self.context.model.profile,
            product: &self.context.model.product,
            bucket: &self.context.model.bucket,
            domain: &self.context.model.domain,
            fingerprint: escrow::fingerprint(&format!(
                "https://{}.r2.cloudflarestorage.com",
                self.context.factory.id()
            )),
            state: if self.action.is_none() {
                "ready"
            } else {
                "pending"
            },
            steps: &self.steps,
            next: self.action,
        };
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("cannot encode authority plan: {error}"))?
            );
        } else {
            println!(
                "{} authority {}: {}",
                report.profile, report.product, report.state
            );
            println!("endpoint fingerprint {}", report.fingerprint);
            for step in report.steps {
                let state = match step.status {
                    plan::Status::Ready => "observed",
                    plan::Status::Change => "change",
                    plan::Status::Deferred => "deferred",
                };
                println!("{state} {}\n  {}", step.resource, step.detail);
            }
        }
        Ok(())
    }

    fn apply(&self) -> Result<(), String> {
        let action = self
            .action
            .ok_or_else(|| "release authority has no pending action".to_string())?;
        let fresh = Self::inspect(self.context.model.clone())?;
        if fresh.seen != self.seen || fresh.action != self.action {
            return Err("release authority plan became stale".into());
        }
        self.context.apply(action)
    }
}
