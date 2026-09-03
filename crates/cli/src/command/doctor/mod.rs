use crate::catalog::model::Coverage;
use crate::catalog::rules::depot::DEPOT_SCHEMA;
use crate::command::depot;
use crate::command::precommit;
use crate::judge::{self, finding};
use crate::shape;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub(crate) mod dependency;
mod human;

#[derive(Serialize)]
struct Report {
    operation: &'static str,
    target: String,
    ok: bool,
    clean: bool,
    configuration: Option<String>,
    profile: Option<String>,
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
    digest: Option<String>,
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
    let mut configuration = None;
    let mut profile = None;
    let mut view = None;
    let mut governance = Vec::new();
    match governed(&root) {
        Ok(Some(held)) => {
            configuration = Some(held.configuration.clone());
            profile = Some(held.digest.clone());
            if root.join("plumb.toml").exists() || root.join("ectropy.toml").exists() {
                governance.push(finding::Finding::new(finding::Seed::wrong(
                    &DEPOT_SCHEMA,
                    "a Depot-governed product must not carry plumb.toml or ectropy.toml",
                )));
            }
            match crate::command::precommit::tree::Index::working(&root)
                .and_then(|index| index.govern(&held).map(|()| index))
            {
                Ok(index) => view = Some(index),
                Err(error) => governance.push(finding::Finding::new(finding::Seed::blind(
                    &DEPOT_SCHEMA,
                    error,
                ))),
            }
        }
        Ok(None) => {}
        Err(error) => governance.push(finding::Finding::new(finding::Seed::blind(
            &DEPOT_SCHEMA,
            error,
        ))),
    }
    let observed = view
        .as_ref()
        .map_or(root.as_path(), |index| index.root.as_path());
    let mut held = shape::capture(observed, &snapshot);
    dependency::observe(&mut held.dependencies, observed, line(observed).as_deref());
    let vocabulary = match &snapshot {
        Ok(snapshot) => plumb::vocabulary::observe(snapshot),
        Err(error) => Err(error.clone()),
    };
    let depot = depot::observe(&snapshot);
    let mut findings = governance;
    findings.extend(judge::judge(&held));
    if profile.is_some() || root.join("plumb.toml").is_file() {
        findings.extend(precommit::hooks(&root).into_iter().map(|held| match held {
            precommit::hook::Finding::Wrong(evidence) => {
                finding::Finding::new(finding::Seed::wrong(&DEPOT_SCHEMA, evidence))
            }
            precommit::hook::Finding::Blind(evidence) => {
                finding::Finding::new(finding::Seed::blind(&DEPOT_SCHEMA, evidence))
            }
        }));
    }
    findings.extend(judge::vocabulary::judge(&vocabulary));
    findings.extend(judge::depot::judge(&depot));
    findings.extend(judge::depot::runtime());
    let summary = Summary::new(&findings);
    let ok = summary.wrong == 0 && summary.blind == 0;
    if json {
        let report = Report {
            operation: "doctor",
            target: root.display().to_string(),
            ok,
            clean: findings.is_empty(),
            configuration: configuration.clone(),
            profile: profile.clone(),
            shape: Shape::new(&held),
            vocabulary: Vocabulary::new(vocabulary),
            findings,
            briefs: briefs(),
            summary,
            coverage: crate::catalog::coverage(),
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
            configuration: configuration.as_deref(),
            profile: profile.as_deref(),
        });
    }
    i32::from(!ok)
}

fn governed(root: &Path) -> Result<Option<shape::product::Profile>, String> {
    shape::product::governance(root).map(|target| target.and_then(|target| target.profile))
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
                digest: Some(report.digest),
                retired: Some(report.retired),
                coverage: Some(report.coverage),
                hits: report.hits,
                ok: report.ok,
                refusal: None,
            },
            Err(refusal) => Self {
                schema: plumb::vocabulary::SCHEMA,
                codec: plumb::vocabulary::CODEC,
                digest: None,
                retired: None,
                coverage: None,
                hits: Vec::new(),
                ok: false,
                refusal: Some(refusal),
            },
        }
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
