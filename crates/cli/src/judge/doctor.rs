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
    vocabulary: Vocabulary,
    findings: Vec<finding::Finding>,
    summary: Summary,
    coverage: Coverage,
}

#[derive(Serialize)]
struct Vocabulary {
    schema: &'static str,
    codec: &'static str,
    dictionary_digest: Option<String>,
    retired: Option<usize>,
    coverage: Option<plumb::vocabulary::Coverage>,
    hits: Vec<plumb::vocabulary::Hit>,
    ok: bool,
    refusal: Option<plumb::vocabulary::Refusal>,
}

#[derive(Serialize)]
struct Shape {
    wrappers: Vec<String>,
    layout: Vec<String>,
    lanes: Vec<String>,
    publishes: Vec<String>,
    sites: Vec<String>,
    dependencies: Vec<Dependency>,
    law: Law,
    skills: Vec<Skill>,
}

#[derive(Serialize)]
struct Skill {
    name: String,
    source: usize,
    text: usize,
    budget: usize,
    files: usize,
}

#[derive(Serialize)]
struct Dependency {
    ecosystem: &'static str,
    name: String,
    requirement: String,
    pinned: bool,
    resolution: String,
    latest: Option<String>,
    seat: String,
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
    let mut held = shape::read(&root);
    held.dependencies.current();
    let vocabulary = plumb::vocabulary::inspect(&root);
    let mut findings = judge(&held);
    findings.extend(retired(&vocabulary));
    let summary = Summary::new(&findings);
    let ok = summary.wrong == 0 && summary.blind == 0;
    if json {
        let report = Report {
            operation: "doctor",
            target: root.display().to_string(),
            ok,
            clean: findings.is_empty(),
            shape: Shape::new(&held),
            vocabulary: Vocabulary::new(vocabulary),
            findings,
            summary,
            coverage: catalog::coverage(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("doctor report should encode")
        );
    } else {
        human(&root, &held, &vocabulary, &findings);
    }
    i32::from(!ok)
}

impl Vocabulary {
    fn new(result: Result<plumb::vocabulary::Report, plumb::vocabulary::Refusal>) -> Self {
        match result {
            Ok(report) => Self {
                schema: report.schema,
                codec: report.codec,
                dictionary_digest: Some(report.dictionary_digest),
                retired: Some(report.retired),
                coverage: Some(report.coverage),
                hits: report.hits,
                ok: report.ok,
                refusal: None,
            },
            Err(refusal) => Self {
                schema: plumb::vocabulary::SCHEMA,
                codec: plumb::vocabulary::CODEC,
                dictionary_digest: None,
                retired: None,
                coverage: None,
                hits: Vec::new(),
                ok: false,
                refusal: Some(refusal),
            },
        }
    }
}

fn retired(
    result: &Result<plumb::vocabulary::Report, plumb::vocabulary::Refusal>,
) -> Vec<finding::Finding> {
    use super::catalog::rules::vocabulary::RETIRED_TERM_ABSENT;

    match result {
        Ok(report) => report
            .hits
            .iter()
            .map(|hit| {
                finding::Finding::new(finding::wrong(
                    &RETIRED_TERM_ABSENT,
                    format!(
                        "{} contains retired domain term {} in {}",
                        hit.path, hit.term, hit.surface
                    ),
                ))
            })
            .collect(),
        Err(error) => vec![finding::Finding::new(finding::blind(
            &RETIRED_TERM_ABSENT,
            error.to_string(),
        ))],
    }
}

impl Shape {
    fn new(held: &shape::Shape) -> Self {
        Self {
            wrappers: held.wrappers.iter().cloned().collect(),
            layout: held.dirs.iter().cloned().collect(),
            lanes: held.lanes.iter().cloned().collect(),
            publishes: held.ships.iter().cloned().collect(),
            sites: held.sites.iter().cloned().collect(),
            dependencies: held
                .dependencies
                .held
                .iter()
                .map(|dependency| Dependency {
                    ecosystem: dependency.ecosystem.name(),
                    name: dependency.name.clone(),
                    requirement: dependency.requirement.clone(),
                    pinned: dependency.pinned,
                    resolution: dependency.resolution.clone(),
                    latest: dependency.latest.clone(),
                    seat: dependency.seat.clone(),
                })
                .collect(),
            law: Law {
                block: held.block.unwrap_or(0),
                path: held.path.unwrap_or(0),
                grants: held.grants.iter().cloned().collect(),
            },
            skills: held
                .skills
                .iter()
                .map(|skill| Skill {
                    name: skill.name.clone(),
                    source: skill.source,
                    text: skill.text,
                    budget: skill.budget,
                    files: skill.files,
                })
                .collect(),
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

fn human(
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
    for skill in &held.skills {
        println!(
            "  skill     {} source={} budget={} text={} files={}",
            skill.name, skill.source, skill.budget, skill.text, skill.files
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
        "  {} out of true, {} unknown to the skeleton, {} blind",
        summary.wrong, summary.unknown, summary.blind
    );
}
