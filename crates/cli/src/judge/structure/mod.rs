use super::finding::{Found, blind, unknown, wrong};
use super::text::{COMPONENTS, CONCURRENCY};
use crate::catalog::rules::structure as rule;
use crate::catalog::set::RULES;
use crate::shape;
use std::collections::BTreeSet;

mod release;

pub fn judge(held: &shape::Shape) -> Found {
    Structure(held).judge()
}

struct Structure<'a>(&'a shape::Shape);

impl Structure<'_> {
    fn judge(&self) -> Found {
        let held = self.0;
        let mut found = Found::new();
        if held.runseal {
            let guarded = !held.guards.is_empty();
            if held.guards.is_empty() {
                found.push(wrong(&rule::GUARD_LANE_PRESENT, "no guard workflow"));
            }
            if held.guards.len() > 1 {
                found.push(wrong(
                    &rule::GUARD_LANE_PRESENT,
                    format!(
                        "multiple guard workflows: {}",
                        held.guards
                            .iter()
                            .map(|(seat, _)| seat.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            if !held.laws {
                found.push(wrong(&rule::ECTROPY_POLICY_PRESENT, "no ectropy.toml"));
            }
            if guarded && !(held.guard.contains("plumb") && held.guard.contains("doctor")) {
                found.push(wrong(
                    &rule::GUARD_RUNS_DOCTOR,
                    "guard does not run plumb doctor",
                ));
            }
            if guarded && !held.guard.contains("ectropy") {
                found.push(wrong(
                    &rule::GUARD_RUNS_ECTROPY,
                    "guard does not run ectropy explicitly",
                ));
            }
            if held.rust && guarded && !held.guard.contains("--release") {
                found.push(wrong(
                    &rule::GUARD_CHECKS_RELEASE_PROFILE,
                    "guard does not exercise the release profile",
                ));
            }
            if guarded && (held.guard.contains("--strict") || held.guard.contains("--debt")) {
                found.push(wrong(
                    &rule::GUARD_USES_CURRENT_ECTROPY_MODE,
                    "guard uses an obsolete ectropy mode",
                ));
            }
            for (seat, workflow) in &held.guards {
                if !workflow.contains(CONCURRENCY) {
                    found.push(wrong(
                        &rule::GUARD_CONCURRENCY,
                        format!("{seat} lacks the concurrency block"),
                    ));
                }
            }
        }
        if let Some(name) = &held.mint {
            found.push(wrong(
                &rule::PACKAGE_UNDER_PACKAGES,
                format!("publishable package {name} at the root, not under packages/"),
            ));
        }
        for (dir, name) in &held.packages {
            let last = name.rsplit('/').next().unwrap_or(name);
            if last != dir {
                found.push(wrong(
                    &rule::PACKAGE_DIRECTORY_NAME,
                    format!(
                        "package {name} sits in packages/{dir}, the directory must match the name"
                    ),
                ));
            }
        }
        if held.components {
            found.push(wrong(&rule::RESERVED_COMPONENTS_SEAT, COMPONENTS));
        }
        if let Some(why) = &held.unread {
            found.push(blind(
                &rule::ECTROPY_POLICY_READABLE,
                format!(
                    "cannot read ectropy.toml: {}",
                    why.lines().next().unwrap_or("")
                ),
            ));
        } else {
            for line in &held.policy {
                found.push(wrong(&rule::ECTROPY_POLICY, line.clone()));
            }
        }
        if self.declared() {
            found.extend(held.layout.found.clone());
        } else {
            for name in &held.dirs {
                if !RULES.dirs.contains(name) {
                    found.push(unknown(
                        &rule::KNOWN_DIRECTORY,
                        format!("directory {name} has no shadow in the skeleton"),
                    ));
                }
            }
        }
        found.extend(release::judge(held));
        self.matched(&mut found);
        self.anchored(&mut found);
        for name in &held.lanes {
            if !RULES.lanes.contains(name) {
                found.push(unknown(
                    &rule::KNOWN_WORKFLOW,
                    format!("workflow {name} has no shadow in the skeleton"),
                ));
            }
        }
        found
    }

    fn declared(&self) -> bool {
        matches!(
            self.0.layout.held,
            shape::layout::Held::Stated(_) | shape::layout::Held::Wrong(_)
        )
    }

    fn matched(&self, found: &mut Found) {
        for (path, exists) in &self.0.bounds {
            if !exists {
                found.push(wrong(
                    &rule::BOUNDARY_EXISTS,
                    format!("boundary names {path} which does not exist"),
                ));
            }
        }
    }

    fn anchored(&self, found: &mut Found) {
        let held = self.0;
        if !held.rust {
            return;
        }
        let Some(repo) = &held.repo else {
            return;
        };
        let anchors: Vec<&String> = held
            .named
            .iter()
            .filter(|(_, name)| name == repo)
            .map(|(seat, _)| seat)
            .collect();
        if anchors.is_empty() {
            found.push(wrong(
                &rule::ANCHOR_PRESENT,
                format!("no crate is named {repo}, the anchor is missing"),
            ));
        }
        if anchors.len() > 1 {
            found.push(wrong(
                &rule::ANCHOR_UNIQUE,
                format!(
                    "{} crates are named {repo}, the anchor must be alone",
                    anchors.len()
                ),
            ));
        }
        let mut seats: BTreeSet<&str> = anchors.iter().map(|seat| seat.as_str()).collect();
        for (seat, _) in held.named.iter().filter(|(_, name)| name == "api") {
            if held.entries.contains(seat) {
                seats.insert(seat);
            } else {
                found.push(wrong(
                    &rule::API_ENTRYPOINT,
                    format!("api crate at {seat} is headless"),
                ));
            }
        }
        for seat in &held.derived {
            if !seats.contains(seat.as_str()) {
                found.push(wrong(
                    &rule::CASCADE_DERIVES_IN_ANCHOR,
                    format!("Cascade derives in {seat}, outside the anchor"),
                ));
            }
        }
    }
}
