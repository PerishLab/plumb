use plumb::snapshot::{Refusal, Snapshot};
use std::collections::BTreeSet;
use std::path::Path;

mod check;
mod config;
mod grammar;
mod seal;

pub use check::check;
pub use config::{Config, Strategy};

pub struct Read {
    pub config: Config,
    pub held: Vec<Document>,
    pub conflicts: Vec<String>,
    pub blind: Option<String>,
}

pub struct Document {
    pub strategy: Strategy,
    pub target: String,
    pub expected: String,
    pub actual: Option<String>,
    pub sources: Vec<Source>,
    pub lines: Option<usize>,
    pub budget: Option<usize>,
    pub leaves: Option<usize>,
    pub errors: Vec<String>,
}

pub struct Source {
    pub path: String,
    pub expected: String,
    pub actual: Option<String>,
    pub lines: Option<usize>,
    pub leaves: Option<usize>,
    pub error: Option<String>,
    pub loose: Vec<String>,
}

pub fn read(root: &Path, snapshot: Result<&Snapshot, &Refusal>) -> Read {
    let config = config::read(root);
    let mut found = Read {
        config,
        held: Vec::new(),
        conflicts: Vec::new(),
        blind: None,
    };
    let Config::Held(bindings) = &found.config else {
        return found;
    };
    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            found.blind = Some(error.to_string());
            return found;
        }
    };
    let documents = bindings
        .iter()
        .flat_map(|binding| binding.strategy.targets(binding.name.as_deref()))
        .collect::<BTreeSet<_>>();
    let closure = seal::Closure::new(snapshot);
    found.conflicts = snapshot
        .entries()
        .iter()
        .filter(|entry| markdown(entry.path()))
        .filter(|entry| !admitted(entry.path(), &documents))
        .map(|entry| entry.path().to_string())
        .collect();
    for binding in bindings {
        let mut document = Document {
            strategy: binding.strategy,
            target: binding.strategy.target(binding.name.as_deref()),
            expected: binding.target.clone(),
            actual: None,
            sources: Vec::new(),
            lines: None,
            budget: None,
            leaves: None,
            errors: closure.form(binding),
        };
        for source in &binding.sources {
            let loose = closure.loose(&source.path, &documents);
            match closure.source(&source.path, &documents) {
                Ok(metric) => document.sources.push(Source {
                    path: source.path.clone(),
                    expected: source.seal.clone(),
                    actual: Some(metric.seal),
                    lines: Some(metric.lines),
                    leaves: Some(metric.leaves),
                    error: None,
                    loose,
                }),
                Err(error) => document.sources.push(Source {
                    path: source.path.clone(),
                    expected: source.seal.clone(),
                    actual: None,
                    lines: None,
                    leaves: None,
                    error: Some(error),
                    loose,
                }),
            }
        }
        match closure.target(binding) {
            Ok(target) => {
                document.actual = Some(target.seal);
                document.lines = Some(target.lines);
                document.leaves = Some(target.leaves);
                document.target = label(binding, &target.paths);
            }
            Err(error) => document.errors.push(error),
        }
        document.budget = budget(&document);
        found.held.push(document);
    }
    found
}

pub fn named(loose: &[String]) -> String {
    const SHOWN: usize = 5;

    let head = loose.iter().take(SHOWN).cloned().collect::<Vec<_>>();
    let rest = loose.len().saturating_sub(head.len());
    if rest == 0 {
        head.join(", ")
    } else {
        format!("{} and {rest} more", head.join(", "))
    }
}

fn label(binding: &config::Binding, paths: &[String]) -> String {
    if binding.strategy == Strategy::Brief {
        return binding.strategy.target(binding.name.as_deref());
    }
    paths.join(" ")
}

fn budget(document: &Document) -> Option<usize> {
    match document.strategy {
        Strategy::Agent | Strategy::Design | Strategy::Architecture => {
            let leaves = document
                .sources
                .iter()
                .map(|source| source.leaves)
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .sum::<usize>();
            Some((32 * root(leaves)).clamp(240, 800))
        }
        Strategy::Brief => {
            let lines = document
                .sources
                .iter()
                .map(|source| source.lines)
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .sum::<usize>();
            Some((4 * root(lines)).clamp(240, 800))
        }
    }
}

fn root(value: usize) -> usize {
    (value as f64).sqrt().ceil() as usize
}

fn markdown(path: &str) -> bool {
    path.ends_with(".md")
}

fn admitted(path: &str, documents: &BTreeSet<String>) -> bool {
    documents.contains(path) || mechanism(path)
}

pub(super) fn mechanism(path: &str) -> bool {
    ["docs/CHANGELOG", plumb::datum::HOME]
        .iter()
        .any(|seat| path == *seat || path.starts_with(&format!("{seat}/")))
}
