use super::finding::{Found, blind, unknown, wrong};
use super::text::COMPONENTS;
use crate::catalog::rules::structure as rule;
use crate::catalog::set;
use crate::shape;
use std::collections::BTreeSet;

pub(crate) mod layout;
mod policy;
mod release;

pub fn judge(held: &shape::Shape) -> Found {
    Structure(held).judge()
}

struct Structure<'a>(&'a shape::Shape);

impl Structure<'_> {
    fn judge(&self) -> Found {
        let held = self.0;
        let mut found = Found::new();
        if held.runseal && !held.laws {
            found.push(wrong(&rule::ECTROPY_POLICY_PRESENT, "no ectropy.toml"));
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
        } else if let Some(policy) = &held.policy {
            found.extend(policy::judge(policy));
        }
        if self.declared() {
            found.extend(layout::judge(&held.layout));
        } else {
            for name in &held.dirs {
                if !set::current().dirs.contains(name) {
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
            if !set::current().lanes.contains(name) {
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
