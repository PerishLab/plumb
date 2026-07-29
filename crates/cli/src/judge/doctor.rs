use super::{finding, judge, show};
use crate::shape;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Report {
    operation: &'static str,
    target: String,
    ok: bool,
    clean: bool,
    shape: Shape,
    findings: Vec<finding::Finding>,
    summary: Summary,
}

#[derive(Serialize)]
struct Shape {
    wrappers: Vec<String>,
    layout: Vec<String>,
    lanes: Vec<String>,
    publishes: Vec<String>,
    sites: Vec<String>,
    law: Law,
}

#[derive(Serialize)]
struct Law {
    block: i64,
    path: i64,
    grants: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    out_of_true: usize,
    unknown: usize,
    blind: usize,
}

pub fn run(root: PathBuf, json: bool) -> i32 {
    let held = shape::read(&root);
    let findings = judge(&held);
    let summary = Summary::new(&findings);
    let ok = summary.out_of_true == 0;
    if json {
        let report = Report {
            operation: "doctor",
            target: root.display().to_string(),
            ok,
            clean: findings.is_empty(),
            shape: Shape::new(&held),
            findings,
            summary,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("doctor report should encode")
        );
    } else {
        human(&root, &held, &findings, &summary);
    }
    i32::from(!ok)
}

impl Shape {
    fn new(held: &shape::Shape) -> Self {
        Self {
            wrappers: held.wrappers.iter().cloned().collect(),
            layout: held.dirs.iter().cloned().collect(),
            lanes: held.lanes.iter().cloned().collect(),
            publishes: held.ships.iter().cloned().collect(),
            sites: held.sites.iter().cloned().collect(),
            law: Law {
                block: held.block.unwrap_or(0),
                path: held.path.unwrap_or(0),
                grants: held.grants.iter().cloned().collect(),
            },
        }
    }
}

impl Summary {
    fn new(findings: &[finding::Finding]) -> Self {
        Self {
            out_of_true: findings
                .iter()
                .filter(|finding| finding.grade == "out of true")
                .count(),
            unknown: findings
                .iter()
                .filter(|finding| finding.grade == "unknown shape")
                .count(),
            blind: findings
                .iter()
                .filter(|finding| finding.grade == "blind")
                .count(),
        }
    }
}

fn human(root: &Path, held: &shape::Shape, findings: &[finding::Finding], summary: &Summary) {
    println!("plumb doctor {}", root.display());
    println!();
    println!("  wrappers  {}", show(&held.wrappers));
    println!("  layout    {}", show(&held.dirs));
    println!("  lanes     {}", show(&held.lanes));
    println!("  publishes {}", show(&held.ships));
    if !held.sites.is_empty() {
        println!("  sites     {}", show(&held.sites));
    }
    println!(
        "  law       block={} path={} grants={}",
        held.block.unwrap_or(0),
        held.path.unwrap_or(0),
        show(&held.grants)
    );
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
        "  {} out of true, {} unknown to the skeleton, {} blind",
        summary.out_of_true, summary.unknown, summary.blind
    );
}
