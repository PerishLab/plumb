use super::catalog::model::Coverage;
use super::{catalog, finding, judge};
use crate::shape;
use serde::Serialize;
use std::path::{Path, PathBuf};

mod human;

#[derive(Serialize)]
struct Report {
    operation: &'static str,
    target: String,
    ok: bool,
    clean: bool,
    shape: Shape,
    vocabulary: Vocabulary,
    findings: Vec<finding::Finding>,
    briefs: Vec<String>,
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
    documents: Vec<Document>,
}

#[derive(Serialize)]
struct Document {
    strategy: &'static str,
    target: String,
    source: usize,
    text: Option<usize>,
    budget: Option<usize>,
    leaves: Option<usize>,
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
    noted: usize,
}

pub fn run(root: PathBuf, json: bool) -> i32 {
    let snapshot = plumb::snapshot::Snapshot::read(&root);
    let mut held = shape::capture(&root, &snapshot);
    held.dependencies.judge(&root, line(&root).as_deref());
    let vocabulary = match &snapshot {
        Ok(snapshot) => plumb::vocabulary::observe(snapshot),
        Err(error) => Err(error.clone()),
    };
    let mut findings = judge(&held);
    findings.extend(retired(&vocabulary));
    findings.extend(super::depot::judge(&snapshot));
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
            briefs: briefs(),
            summary,
            coverage: catalog::coverage(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("doctor report should encode")
        );
    } else {
        human::render(human::Held {
            root: &root,
            shape: &held,
            vocabulary: &vocabulary,
            findings: &findings,
            briefs: &briefs(),
        });
    }
    i32::from(!ok)
}

fn briefs() -> Vec<String> {
    let running = plumb::version!("PLUMB");
    let Ok(rig) = plumb::rig::Rig::resolve(None) else {
        return Vec::new();
    };
    let kit = plumb::skill::Kit {
        name: "plumb".to_string(),
        home: plumb::config::home().unwrap_or_default(),
        state: std::path::PathBuf::from(&rig.home)
            .join("state")
            .join("skills.json"),
        url: rig.releases.clone(),
    };
    let Ok(records) = kit.list() else {
        return Vec::new();
    };
    records
        .iter()
        .filter(|record| record.version != running)
        .map(|record| {
            format!(
                "{} {} is {}, beside a running plumb {running}; run plumb skill upgrade",
                record.agent,
                record.path.display(),
                record.version
            )
        })
        .collect()
}

fn line(root: &Path) -> Option<String> {
    let declared = plumb::rig::Rig::resolve(None)
        .map(|rig| rig.release.version)
        .unwrap_or_default();
    plumb::datum::Tree(root).line(&declared)
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
                finding::Finding::new(finding::Seed::wrong(
                    &RETIRED_TERM_ABSENT,
                    format!(
                        "{} contains retired domain term {} in {}",
                        hit.path, hit.term, hit.surface
                    ),
                ))
            })
            .collect(),
        Err(error) => vec![finding::Finding::new(finding::Seed::blind(
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
            documents: held
                .documents
                .held
                .iter()
                .map(|document| Document {
                    strategy: document.strategy.id(),
                    target: document.target.clone(),
                    source: document.sources.len(),
                    text: document.lines,
                    budget: document.budget,
                    leaves: document.leaves,
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
            noted: findings
                .iter()
                .filter(|finding| finding.grade == "noted")
                .count(),
        }
    }
}
