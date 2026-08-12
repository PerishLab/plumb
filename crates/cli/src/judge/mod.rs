use self::finding::{Finding, blind, unknown, wrong};
use crate::{rules::RULES, shape};
use catalog::rules::{env as env_rule, structure as structure_rule};
use shape::Found;
use std::collections::BTreeSet;
use text::{COMPONENTS, CONCURRENCY, CONTAINER};

pub(crate) mod catalog;
mod deps;
pub(crate) mod doctor;
pub(crate) mod finding;
pub(crate) mod precommit;
pub mod radius;
mod skill;
mod text;

pub use text::show;

pub fn judge(held: &shape::Shape) -> Vec<Finding> {
    let mut notes = Vec::new();
    for found in [
        held.env(),
        shape::document::check(held),
        skill::check(held),
        held.structure(),
        deps::check(held),
        held.web.clone().unwrap_or_default(),
        held.dispatch.clone().unwrap_or_default(),
        shape::locked(held),
    ] {
        for seed in found {
            notes.push(Finding::new(seed));
        }
    }
    notes
}

impl shape::Shape {
    fn env(&self) -> Found {
        let mut found = Found::new();
        if self
            .edition
            .as_ref()
            .is_some_and(|edition| edition != "2024")
        {
            found.push(wrong(
                &env_rule::EDITION_2024,
                format!(
                    "edition is {}, the skeleton holds 2024",
                    self.edition.as_deref().unwrap_or("")
                ),
            ));
        }
        let pinned = format!("{CONTAINER}:");
        if self
            .guards
            .iter()
            .any(|(_, workflow)| workflow.contains(&pinned))
        {
            found.push(wrong(
                &env_rule::ROLLING_CI_CONTAINER,
                "CI container pinned to a tag, the skeleton tracks latest".to_string(),
            ));
        }
        found
    }
    fn structure(&self) -> Found {
        let mut found = Found::new();
        if self.runseal {
            let guarded = !self.guards.is_empty();
            if self.guards.is_empty() {
                found.push(wrong(
                    &structure_rule::GUARD_LANE_PRESENT,
                    "no guard workflow",
                ));
            }
            if self.guards.len() > 1 {
                found.push(wrong(
                    &structure_rule::GUARD_LANE_PRESENT,
                    format!(
                        "multiple guard workflows: {}",
                        self.guards
                            .iter()
                            .map(|(seat, _)| seat.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            if !self.laws {
                found.push(wrong(
                    &structure_rule::ECTROPY_POLICY_PRESENT,
                    "no ectropy.toml",
                ));
            }
            if guarded && !(self.guard.contains("plumb") && self.guard.contains("doctor")) {
                found.push(wrong(
                    &structure_rule::GUARD_RUNS_DOCTOR,
                    "guard does not run plumb doctor",
                ));
            }
            if guarded && !self.guard.contains("ectropy") {
                found.push(wrong(
                    &structure_rule::GUARD_RUNS_ECTROPY,
                    "guard does not run ectropy explicitly",
                ));
            }
            if self.rust && guarded && !self.guard.contains("--release") {
                found.push(wrong(
                    &structure_rule::GUARD_CHECKS_RELEASE_PROFILE,
                    "guard does not exercise the release profile",
                ));
            }
            if guarded && (self.guard.contains("--strict") || self.guard.contains("--debt")) {
                found.push(wrong(
                    &structure_rule::GUARD_USES_CURRENT_ECTROPY_MODE,
                    "guard uses an obsolete ectropy mode",
                ));
            }
            for (seat, workflow) in &self.guards {
                if !workflow.contains(CONCURRENCY) {
                    found.push(wrong(
                        &structure_rule::GUARD_CONCURRENCY,
                        format!("{seat} lacks the concurrency block"),
                    ));
                }
            }
        }
        if let Some(name) = &self.mint {
            found.push(wrong(
                &structure_rule::PACKAGE_UNDER_PACKAGES,
                format!("publishable package {name} at the root, not under packages/"),
            ));
        }
        for (dir, name) in &self.packages {
            let last = name.rsplit('/').next().unwrap_or(name);
            if last != dir {
                found.push(wrong(
                    &structure_rule::PACKAGE_DIRECTORY_NAME,
                    format!(
                        "package {name} sits in packages/{dir}, the directory must match the name"
                    ),
                ));
            }
        }
        if self.root.join("packages/components").is_dir() {
            found.push(wrong(&structure_rule::RESERVED_COMPONENTS_SEAT, COMPONENTS));
        }
        if let Some(why) = &self.unread {
            found.push(blind(
                &structure_rule::ECTROPY_POLICY_READABLE,
                format!(
                    "cannot read ectropy.toml: {}",
                    why.lines().next().unwrap_or("")
                ),
            ));
        } else {
            for line in &self.policy {
                found.push(wrong(&structure_rule::ECTROPY_POLICY, line.clone()));
            }
        }
        for name in &self.dirs {
            if !RULES.dirs.contains(name) && !self.actions.contains(name) {
                found.push(unknown(
                    &structure_rule::KNOWN_DIRECTORY,
                    format!("directory {name} has no shadow in the skeleton"),
                ));
            }
        }
        found.extend(shape::pair::judge(self));
        self.matched(&mut found);
        self.anchored(&mut found);
        for name in &self.lanes {
            if !RULES.lanes.contains(name) {
                found.push(unknown(
                    &structure_rule::KNOWN_WORKFLOW,
                    format!("workflow {name} has no shadow in the skeleton"),
                ));
            }
        }
        found
    }
    fn matched(&self, found: &mut Found) {
        for path in &self.bounds {
            if !self.root.join(path).exists() {
                found.push(wrong(
                    &structure_rule::BOUNDARY_EXISTS,
                    format!("boundary names {path} which does not exist"),
                ));
            }
        }
    }

    fn anchored(&self, found: &mut Found) {
        if !self.rust {
            return;
        }
        let Some(repo) = &self.repo else {
            return;
        };
        let anchors: Vec<&String> = self
            .named
            .iter()
            .filter(|(_, name)| name == repo)
            .map(|(seat, _)| seat)
            .collect();
        if anchors.is_empty() {
            found.push(wrong(
                &structure_rule::ANCHOR_PRESENT,
                format!("no crate is named {repo}, the anchor is missing"),
            ));
        }
        if anchors.len() > 1 {
            found.push(wrong(
                &structure_rule::ANCHOR_UNIQUE,
                format!(
                    "{} crates are named {repo}, the anchor must be alone",
                    anchors.len()
                ),
            ));
        }
        let mut seats: BTreeSet<&str> = anchors.iter().map(|seat| seat.as_str()).collect();
        for (seat, _) in self.named.iter().filter(|(_, name)| name == "api") {
            if self.entries.contains(seat) {
                seats.insert(seat);
            } else {
                found.push(wrong(
                    &structure_rule::API_ENTRYPOINT,
                    format!("api crate at {seat} is headless"),
                ));
            }
        }
        for seat in &self.derived {
            if !seats.contains(seat.as_str()) {
                found.push(wrong(
                    &structure_rule::CASCADE_DERIVES_IN_ANCHOR,
                    format!("Cascade derives in {seat}, outside the anchor"),
                ));
            }
        }
    }
}
