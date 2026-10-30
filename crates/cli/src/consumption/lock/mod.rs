use clap::Subcommand;
use plumb::cli::Root;
use record::{Inspection, Skip, Summary, Tool};
use std::collections::BTreeMap;
use std::path::Path;

mod crates;
mod form;
mod items;
mod model;
mod record;
mod settle;
mod source;
mod tree;
mod walk;

#[derive(Subcommand)]
pub enum Deed {
    #[command(
        about = "Inventory every lock use in a repository with resolved roles, holds, and gaps"
    )]
    Inspect {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
}

pub fn run(deed: Deed) -> i32 {
    let Deed::Inspect { target, json } = deed;
    let result = inspect(Path::new(&target.root)).and_then(|held| render(&held, json));
    match result {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            eprintln!("plumb lock: {error}");
            2
        }
    }
}

pub fn inspect(root: &Path) -> Result<Inspection, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    let checkout = source::Checkout { root };
    let source = checkout.read();
    let packages = checkout.packages(&source.files);
    let mut model = model::Model::default();
    let mut forest = tree::Forest::default();
    let mut skips = source::foreign(&source.files);
    for package in &packages {
        let record = &package.record;
        let Some(directory) = &package.directory else {
            if record.source == crate::catalog::set::CARGO.index {
                skips.push(Skip {
                    path: format!("{}-{}", record.name, record.version),
                    reason: "first-party dependency source is absent from the local Cargo registry"
                        .into(),
                });
            }
            continue;
        };
        let label = format!("{}-{}", record.name, record.version);
        let held = crates::Package {
            disk: directory,
            shown: &label,
            local: false,
        };
        for target in crates::targets(&held) {
            forest.plant(&mut model, &target);
        }
    }
    for manifest in source
        .files
        .iter()
        .filter(|path| path.rsplit('/').next() == Some("Cargo.toml"))
    {
        let shown = manifest
            .strip_suffix("Cargo.toml")
            .unwrap_or_default()
            .trim_end_matches('/');
        let disk = root.join(shown);
        let held = crates::Package {
            disk: &disk,
            shown,
            local: true,
        };
        for mut target in crates::targets(&held) {
            if fixture(shown) {
                target.context = "fixture";
            }
            forest.plant(&mut model, &target);
        }
    }
    for path in source.files.iter().filter(|path| path.ends_with(".rs")) {
        if !forest.reached.contains(path) {
            forest.plant(&mut model, &detached(root, path));
        }
    }
    items::sign(&mut model, &forest.units);
    let mut harvest = walk::walk(&model, &forest);
    let mut unknown = std::mem::take(&mut harvest.gaps);
    unknown.append(&mut forest.gaps);
    unknown.sort();
    unknown.dedup();
    let findings = settle::settle(&model, harvest);
    skips.append(&mut forest.skips);
    skips.sort();
    skips.dedup();
    let mut roles = BTreeMap::new();
    for finding in &findings {
        *roles.entry(finding.role).or_insert(0) += 1;
    }
    let summary = Summary {
        files: source.files.len(),
        rust: source
            .files
            .iter()
            .filter(|path| path.ends_with(".rs"))
            .count(),
        findings: findings.len(),
        roles,
        unknown: unknown.len(),
        unscanned: skips.len(),
        complete: unknown.is_empty() && skips.is_empty(),
    };
    Ok(Inspection {
        schema: record::SCHEMA,
        tool: Tool {
            name: "plumb",
            version: plumb::version!("PLUMB").to_string(),
            parser: "syn 2",
        },
        repository: source.repository,
        dependencies: packages.into_iter().map(|package| package.record).collect(),
        findings,
        unknown,
        unscanned: skips,
        limits: record::LIMITS.to_vec(),
        summary,
    })
}

fn fixture(shown: &str) -> bool {
    shown
        .split('/')
        .any(|part| matches!(part, "tests" | "fixtures" | "fixture" | "testdata"))
}

fn detached(root: &Path, path: &str) -> crates::Target {
    crates::Target {
        key: format!("{path}[detached]"),
        entry: root.join(path),
        base: root.to_path_buf(),
        label: String::new(),
        context: "detached",
        externs: BTreeMap::new(),
        local: true,
    }
}

fn render(held: &Inspection, json: bool) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(held).map_err(|error| error.to_string());
    }
    let mut lines = Vec::new();
    for finding in &held.findings {
        let lifecycle = finding
            .lifecycle
            .map(|name| format!("[{name}]"))
            .unwrap_or_default();
        let call = finding
            .call
            .clone()
            .or_else(|| finding.kind.clone())
            .unwrap_or_default();
        let hold = finding
            .hold
            .as_ref()
            .map(|hold| format!(" hold={}", hold.form))
            .unwrap_or_default();
        let cfg = match finding.cfg.as_slice() {
            [] => String::new(),
            list => format!(" cfg({})", list.join(", ")),
        };
        lines.push(format!(
            "{}:{}:{} {}{lifecycle} {call}{hold}{cfg} ({})",
            finding.path, finding.line, finding.column, finding.role, finding.context
        ));
    }
    for gap in &held.unknown {
        lines.push(format!(
            "{}:{}:{} unknown {}",
            gap.path, gap.line, gap.column, gap.reason
        ));
    }
    for skip in &held.unscanned {
        lines.push(format!("{} unscanned {}", skip.path, skip.reason));
    }
    let summary = &held.summary;
    let state = if summary.complete {
        "complete"
    } else {
        "incomplete"
    };
    lines.push(format!(
        "lock inspection {state}: {} findings, {} unknown, {} unscanned across {} Rust files",
        summary.findings, summary.unknown, summary.unscanned, summary.rust
    ));
    Ok(lines.join("\n"))
}
