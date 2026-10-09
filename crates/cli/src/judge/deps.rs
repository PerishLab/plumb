use super::finding::{Found, blind, wrong};
use crate::catalog::rules::deps as rule;
use crate::catalog::rules::release::DATUM_RECORDED;
use crate::catalog::set;
use crate::shape::{self, Dependency};
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
    if let Some(version) = &held.dependencies.missing {
        found.push(wrong(
            &DATUM_RECORDED,
            format!("release line {version} records no datum to judge against"),
        ));
    }
    for error in &held.dependencies.blind {
        found.push(blind(&rule::FIRST_PARTY_STABLE_LATEST, error.clone()));
    }
    for dependency in &held.dependencies.held {
        currency(dependency, &mut found);
    }
    loaders(&held.dependencies, &mut found);
    for (seat, name) in &held.node {
        if set::current().blacklist.contains(name) {
            found.push(wrong(
                &rule::STYLING_PACKAGE_ALLOWED,
                format!("{seat} depends on blacklisted styling package {name}"),
            ));
        }
    }
    found
}

fn loaders(held: &shape::Dependencies, found: &mut Found) {
    for (seat, name) in &held.direct {
        if set::current().refused.contains(name) {
            found.push(wrong(
                &rule::LOADER_ABSENT,
                format!(
                    "{seat} depends on .env loader {name}; declare settings through plumb Cascade"
                ),
            ));
        }
    }
    for error in &held.unread {
        found.push(blind(&rule::LOADER_ABSENT, error.clone()));
    }
}

const SUBSTRATE: [&str; 2] = ["plumb", "@perishlab/plumb"];

fn currency(dependency: &Dependency, found: &mut Found) {
    if let Some((_, current)) = set::current()
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
            Verdict::Stale { .. } if SUBSTRATE.contains(&dependency.name.as_str()) => {}
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
