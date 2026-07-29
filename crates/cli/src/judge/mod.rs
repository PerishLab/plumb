use self::finding::{Finding, Seed};
use crate::{rules::RULES, shape};
use shape::Found;
use std::collections::BTreeSet;

pub(crate) mod doctor;
pub(crate) mod finding;

const CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";
const CONTAINER: &str = "mirror.perish.lan/ci/deno";
const COMPONENTS: &str =
    "packages/components is reserved; reusable components belong to the design system";

pub fn judge(held: &shape::Shape) -> Vec<Finding> {
    let mut notes = Vec::new();
    for (dim, found) in [
        ("env", held.env()),
        ("structure", held.structure()),
        ("deps", held.deps()),
        ("web", held.web.clone().unwrap_or_default()),
        ("dispatch", held.dispatch.clone().unwrap_or_default()),
        ("lock", shape::locked(held)),
    ] {
        for seed in found {
            notes.push(Finding::new(dim, seed));
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
                "edition-2024",
                format!(
                    "edition is {}, the skeleton holds 2024",
                    self.edition.as_deref().unwrap_or("")
                ),
            ));
        }
        let pinned = format!("{CONTAINER}:");
        if self.lane.as_ref().is_some_and(|yml| yml.contains(&pinned)) {
            found.push(wrong(
                "rolling-ci-container",
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
                    found.push(wrong("missing-wrapper", format!("no {name} wrapper")));
                }
            }
            for name in &RULES.hooks {
                if !self.hooks.contains(name) {
                    found.push(wrong("missing-hook", format!("no {name} hook")));
                }
            }
            if !self.laws {
                found.push(wrong("ectropy-policy-present", "no ectropy.toml"));
            }
            if self.wrappers.contains("guard")
                && !(self.guard.contains("plumb") && self.guard.contains("doctor"))
            {
                found.push(wrong(
                    "guard-runs-doctor",
                    "guard does not run plumb doctor",
                ));
            }
            if self.wrappers.contains("init") && !self.init.contains("\"plumb\"") {
                found.push(wrong("init-requires-plumb", "init does not require plumb"));
            }
            if self
                .lane
                .as_ref()
                .is_some_and(|yml| !yml.contains(CONCURRENCY))
            {
                found.push(wrong(
                    "guard-concurrency",
                    "guard lane without the concurrency block".to_string(),
                ));
            }
        }
        if let Some(name) = &self.mint {
            found.push(wrong(
                "package-under-packages",
                format!("publishable package {name} at the root, not under packages/"),
            ));
        }
        for (dir, name) in &self.packages {
            let last = name.rsplit('/').next().unwrap_or(name);
            if last != dir {
                found.push(wrong(
                    "package-directory-name",
                    format!(
                        "package {name} sits in packages/{dir}, the directory must match the name"
                    ),
                ));
            }
        }
        if self.root.join("packages/components").is_dir() {
            found.push(wrong("reserved-components-seat", COMPONENTS));
        }
        if let Some(why) = &self.unread {
            found.push(blind(
                "ectropy-policy-readable",
                format!(
                    "cannot read ectropy.toml: {}",
                    why.lines().next().unwrap_or("")
                ),
            ));
        } else {
            for line in &self.policy {
                found.push(wrong("ectropy-policy", line.clone()));
            }
        }
        for name in &self.dirs {
            if !RULES.dirs.contains(name) && !self.actions.contains(name) {
                found.push(unknown(
                    "known-directory",
                    format!("directory {name} has no shadow in the skeleton"),
                ));
            }
        }
        for path in &self.operator_tests {
            found.push(wrong(
                "operator-test-owned-by-sealkit",
                format!("{path} is a .runseal test; tested logic belongs in sealkit"),
            ));
        }
        for name in &self.wrappers {
            if !RULES.wrappers.contains(name) {
                found.push(unknown(
                    "known-wrapper",
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
                    "known-workflow",
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
                "rust-binary-uses-clap",
                "ships a rust binary without clap".to_string(),
            ));
        }
        if self.binary && !self.substrate {
            found.push(wrong(
                "rust-binary-uses-plumb",
                "ships a rust binary without plumb".to_string(),
            ));
        }
        for (name, held) in &RULES.retired {
            if self.deno.contains(name.as_str()) {
                found.push(wrong(
                    "current-dependency-name",
                    format!("depends on {name}, renamed to {held}"),
                ));
            }
        }
        for name in RULES.pinned(&self.deno) {
            found.push(wrong(
                "self-built-dependency-unpinned",
                format!("self-built {name} is version-pinned, the skeleton tracks latest"),
            ));
        }
        for (seat, name) in &self.node {
            if RULES.blacklist.contains(name) {
                found.push(wrong(
                    "styling-package-allowed",
                    format!("{seat} depends on blacklisted styling package {name}"),
                ));
            }
        }
        found
    }

    fn matched(&self, found: &mut Found) {
        if self.inits && self.listed.is_empty() {
            found.push(blind(
                "init-paths-readable",
                "an init wrapper is present but its required paths could not be read".to_string(),
            ));
        }
        for name in self.listed.difference(&self.wrappers) {
            found.push(wrong(
                "init-requires-existing-wrapper",
                format!("init requires wrapper {name} which does not exist"),
            ));
        }
        for path in &self.bounds {
            if !self.root.join(path).exists() {
                found.push(wrong(
                    "boundary-exists",
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
                "anchor-present",
                format!("no crate is named {repo}, the anchor is missing"),
            ));
        }
        if anchors.len() > 1 {
            found.push(wrong(
                "anchor-unique",
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
                    "api-entrypoint",
                    format!("api crate at {seat} is headless"),
                ));
            }
        }
        for seat in &self.derived {
            if !seats.contains(seat.as_str()) {
                found.push(wrong(
                    "cascade-derives-in-anchor",
                    format!("Cascade derives in {seat}, outside the anchor"),
                ));
            }
        }
    }
}

pub fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}

fn wrong(code: &'static str, evidence: impl Into<String>) -> Seed {
    Seed::wrong(code, evidence)
}

fn blind(code: &'static str, evidence: impl Into<String>) -> Seed {
    Seed::blind(code, evidence)
}

fn unknown(code: &'static str, evidence: impl Into<String>) -> Seed {
    Seed::unknown(code, evidence)
}
