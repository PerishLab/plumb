use super::catalog::rules::deps as rule;
use super::finding::{blind, wrong};
use crate::rules::RULES;
use crate::shape::{self, Dependency, Found};
use plumb_cli::Verdict;

pub fn check(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.binary && !held.clap {
        found.push(wrong(
            &rule::RUST_BINARY_USES_CLAP,
            "ships a rust binary without clap",
        ));
    }
    if held.binary && !held.substrate {
        found.push(wrong(
            &rule::RUST_BINARY_USES_PLUMB,
            "ships a rust binary without plumb",
        ));
    }
    for error in &held.dependencies.blind {
        found.push(blind(&rule::FIRST_PARTY_STABLE_LATEST, error.clone()));
    }
    for dependency in &held.dependencies.held {
        currency(dependency, &mut found);
    }
    for (seat, name) in &held.node {
        if RULES.blacklist.contains(name) {
            found.push(wrong(
                &rule::STYLING_PACKAGE_ALLOWED,
                format!("{seat} depends on blacklisted styling package {name}"),
            ));
        }
    }
    found
}

fn currency(dependency: &Dependency, found: &mut Found) {
    if let Some((_, current)) = RULES
        .retired
        .iter()
        .find(|(name, _)| name == &dependency.name)
    {
        found.push(wrong(
            &rule::CURRENT_DEPENDENCY_NAME,
            format!("depends on {}, renamed to {current}", dependency.name),
        ));
        return;
    }
    for verdict in plumb_cli::judge(dependency) {
        match verdict {
            Verdict::Pinned => found.push(wrong(
                &rule::SELF_BUILT_DEPENDENCY_UNPINNED,
                format!(
                    "self-built {} is version-pinned at {} in {}",
                    dependency.name, dependency.requirement, dependency.seat
                ),
            )),
            Verdict::Stale { resolution, latest } => found.push(wrong(
                &rule::FIRST_PARTY_STABLE_LATEST,
                format!(
                    "{} {} resolves to {resolution}, stable latest is {latest} [{}]",
                    dependency.ecosystem.name(),
                    dependency.name,
                    dependency.seat
                ),
            )),
            Verdict::Unread(error) => found.push(blind(
                &rule::FIRST_PARTY_STABLE_LATEST,
                format!(
                    "cannot compare {} {} {error}",
                    dependency.ecosystem.name(),
                    dependency.name
                ),
            )),
        }
    }
}
