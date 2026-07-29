use self::finding::{Finding, blind, unknown, wrong};
use crate::{rules::RULES, shape};
use catalog::rules::{deps as deps_rule, env as env_rule, structure as structure_rule};
use shape::Found;
use std::collections::BTreeSet;
use text::{COMPONENTS, CONCURRENCY, CONTAINER};

pub(crate) mod catalog;
pub(crate) mod doctor;
pub(crate) mod finding;
mod text;

pub use text::show;

pub fn judge(held: &shape::Shape) -> Vec<Finding> {
    let mut notes = Vec::new();
    for found in [
        held.env(),
        held.structure(),
        held.deps(),
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
        if self.lane.as_ref().is_some_and(|yml| yml.contains(&pinned)) {
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
            for name in &RULES.required {
                if !self.wrappers.contains(name) {
                    found.push(wrong(
                        &structure_rule::MISSING_WRAPPER,
                        format!("no {name} wrapper"),
                    ));
                }
            }
            for name in &RULES.hooks {
                if !self.hooks.contains(name) {
                    found.push(wrong(
                        &structure_rule::MISSING_HOOK,
                        format!("no {name} hook"),
                    ));
                }
            }
            if !self.laws {
                found.push(wrong(
                    &structure_rule::ECTROPY_POLICY_PRESENT,
                    "no ectropy.toml",
                ));
            }
            if self.wrappers.contains("guard")
                && !(self.guard.contains("plumb") && self.guard.contains("doctor"))
            {
                found.push(wrong(
                    &structure_rule::GUARD_RUNS_DOCTOR,
                    "guard does not run plumb doctor",
                ));
            }
            if self.wrappers.contains("guard") && !self.guard.contains("\"ectropy\"") {
                found.push(wrong(
                    &structure_rule::GUARD_RUNS_ECTROPY,
                    "guard does not run ectropy explicitly",
                ));
            }
            if self.wrappers.contains("guard")
                && (self.guard.contains("\"--strict\"") || self.guard.contains("\"--debt\""))
            {
                found.push(wrong(
                    &structure_rule::GUARD_USES_CURRENT_ECTROPY_MODE,
                    "guard uses an obsolete ectropy mode",
                ));
            }
            if self.wrappers.contains("init") && !self.init.contains("\"plumb\"") {
                found.push(wrong(
                    &structure_rule::INIT_REQUIRES_PLUMB,
                    "init does not require plumb",
                ));
            }
            if self
                .lane
                .as_ref()
                .is_some_and(|yml| !yml.contains(CONCURRENCY))
            {
                found.push(wrong(
                    &structure_rule::GUARD_CONCURRENCY,
                    "guard lane without the concurrency block".to_string(),
                ));
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
        for path in &self.tests {
            found.push(wrong(
                &structure_rule::OPERATOR_TEST_OWNED_BY_SEALKIT,
                format!("{path} is a .runseal test; tested logic belongs in sealkit"),
            ));
        }
        for name in &self.wrappers {
            if !RULES.wrappers.contains(name) {
                found.push(unknown(
                    &structure_rule::KNOWN_WRAPPER,
                    format!("wrapper {name} has no shadow in the skeleton"),
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
    fn deps(&self) -> Found {
        let mut found = Found::new();
        if self.binary && !self.clap {
            found.push(wrong(
                &deps_rule::RUST_BINARY_USES_CLAP,
                "ships a rust binary without clap".to_string(),
            ));
        }
        if self.binary && !self.substrate {
            found.push(wrong(
                &deps_rule::RUST_BINARY_USES_PLUMB,
                "ships a rust binary without plumb".to_string(),
            ));
        }
        for (name, held) in &RULES.retired {
            if self.deno.contains(name.as_str()) {
                found.push(wrong(
                    &deps_rule::CURRENT_DEPENDENCY_NAME,
                    format!("depends on {name}, renamed to {held}"),
                ));
            }
        }
        for name in RULES.pinned(&self.deno) {
            found.push(wrong(
                &deps_rule::SELF_BUILT_DEPENDENCY_UNPINNED,
                format!("self-built {name} is version-pinned, the skeleton tracks latest"),
            ));
        }
        for (seat, name) in &self.node {
            if RULES.blacklist.contains(name) {
                found.push(wrong(
                    &deps_rule::STYLING_PACKAGE_ALLOWED,
                    format!("{seat} depends on blacklisted styling package {name}"),
                ));
            }
        }
        found
    }

    fn matched(&self, found: &mut Found) {
        if self.inits && self.listed.is_empty() {
            found.push(blind(
                &structure_rule::INIT_PATHS_READABLE,
                "an init wrapper is present but its required paths could not be read".to_string(),
            ));
        }
        for name in self.listed.difference(&self.wrappers) {
            found.push(wrong(
                &structure_rule::INIT_REQUIRES_EXISTING_WRAPPER,
                format!("init requires wrapper {name} which does not exist"),
            ));
        }
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
