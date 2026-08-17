use super::super::{finding, show};
use super::Summary;
use crate::shape;
use std::path::Path;

pub fn render(
    root: &Path,
    held: &shape::Shape,
    vocabulary: &Result<plumb::vocabulary::Report, plumb::vocabulary::Refusal>,
    findings: &[finding::Finding],
) {
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
    for document in &held.documents.held {
        println!(
            "  document  {} target={} source={} budget={} text={} leaves={}",
            document.strategy.id(),
            document.target,
            document.sources.len(),
            document
                .budget
                .map_or_else(|| "?".into(), |held| held.to_string()),
            document
                .lines
                .map_or_else(|| "?".into(), |held| held.to_string()),
            document
                .leaves
                .map_or_else(|| "?".into(), |held| held.to_string()),
        );
    }
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
