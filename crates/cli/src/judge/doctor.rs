use super::catalog::model::Coverage;
use super::{catalog, finding, judge, show};
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
    coverage: Coverage,
}

#[derive(Serialize)]
struct Shape {
    wrappers: Vec<String>,
    layout: Vec<String>,
    lanes: Vec<String>,
    publishes: Vec<String>,
    sites: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sealkit: Option<Sealkit>,
    law: Law,
}

#[derive(Serialize)]
struct Sealkit {
    requirement: String,
    resolution: String,
}

#[derive(Serialize)]
struct Law {
    block: i64,
    path: i64,
    grants: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    #[serde(rename = "out_of_true")]
    wrong: usize,
    unknown: usize,
    blind: usize,
}

pub fn run(root: PathBuf, json: bool) -> i32 {
    let held = shape::read(&root);
    let findings = judge(&held);
    let summary = Summary::new(&findings);
    let ok = summary.wrong == 0;
    if json {
        let report = Report {
            operation: "doctor",
            target: root.display().to_string(),
            ok,
            clean: findings.is_empty(),
            shape: Shape::new(&held),
            findings,
            summary,
            coverage: catalog::coverage(),
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
            sealkit: match &held.sealkit {
                shape::Sealkit::Held(dependency) => Some(Sealkit {
                    requirement: dependency.requirement.clone(),
                    resolution: dependency.resolution.clone(),
                }),
                shape::Sealkit::Absent | shape::Sealkit::Blind(_) => None,
            },
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
            wrong: findings
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
    if let shape::Sealkit::Held(dependency) = &held.sealkit {
        println!(
            "  sealkit  {} -> {}",
            dependency.requirement, dependency.resolution
        );
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
        summary.wrong, summary.unknown, summary.blind
    );
}
