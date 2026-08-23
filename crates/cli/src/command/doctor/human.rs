use super::Summary;
use crate::judge::{finding, show};
use crate::shape;
use std::path::Path;

pub struct Held<'a> {
    pub root: &'a Path,
    pub shape: &'a shape::Shape,
    pub vocabulary: &'a Result<plumb::vocabulary::Report, plumb::vocabulary::Refusal>,
    pub findings: &'a [finding::Finding],
    pub briefs: &'a [String],
}

pub fn render(seen: Held<'_>) {
    let Held {
        root,
        shape: held,
        vocabulary,
        findings,
        briefs,
    } = seen;
    let summary = Summary::new(findings);
    println!("plumb doctor {}", root.display());
    println!();
    println!("  wrappers  {}", show(&held.wrappers));
    println!("  layout    {}", show(&held.dirs));
    println!("  lanes     {}", show(&held.lanes));
    println!("  publishes {}", show(&held.ships));
    if !held.sites.is_empty() {
        println!("  sites     {}", show(&held.sites));
    }
    for dependency in &held.dependencies.held {
        println!(
            "  deps      {} {} {} -> {} (latest {}) [{}]",
            dependency.ecosystem.name(),
            dependency.name,
            dependency.requirement,
            dependency.resolution,
            dependency.latest.as_deref().unwrap_or("?"),
            dependency.seat,
        );
    }
    println!(
        "  law       block={} path={} grants={}",
        held.block.unwrap_or(0),
        held.path.unwrap_or(0),
        show(&held.grants)
    );
    match vocabulary {
        Ok(report) => println!(
            "  vocabulary {} {} retired={} scanned={}/{}",
            report.codec,
            &report.dictionary_digest[..12],
            report.retired,
            report.coverage.scanned,
            report.coverage.tracked
        ),
        Err(error) => println!("  vocabulary blind: {error}"),
    }
    for brief in briefs {
        println!("  brief     {brief}");
    }
    println!();
    if findings.is_empty() {
        println!("  true to the skeleton");
        return;
    }
    for finding in findings {
        println!(
            "  {}: {} [{}]",
            finding.grade, finding.evidence, finding.scope
        );
    }
    println!();
    println!(
        "  {} out of true, {} unknown to the skeleton, {} blind, {} noted",
        summary.wrong, summary.unknown, summary.blind, summary.noted
    );
}
