mod cargo;
mod registry;

use crate::catalog::set::RULES;
use plumb::datum::Datum;
use std::path::Path;

pub use plumb_cli::{Dependency, Ecosystem};

#[derive(Default)]
pub struct Dependencies {
    pub held: Vec<Dependency>,
    pub blind: Vec<String>,
    pub missing: Option<String>,
}

impl Dependencies {
    fn normalize(&mut self) {
        self.held.sort();
        self.held.dedup();
        self.blind.sort();
        self.blind.dedup();
    }

    pub fn judge(&mut self, root: &Path, line: Option<&str>) {
        match line {
            Some(version) => self.recorded(root, version),
            None => self.current(),
        }
    }

    pub fn current(&mut self) {
        registry::read(self);
        self.normalize();
    }

    fn recorded(&mut self, root: &Path, version: &str) {
        match plumb::datum::Tree(root).read(version) {
            Ok(Some(datum)) => self.against(&datum),
            Ok(None) => self.missing = Some(version.to_string()),
            Err(error) => self.blind.push(error),
        }
        self.normalize();
    }

    fn against(&mut self, datum: &Datum) {
        let mut blind = Vec::new();
        for dependency in &mut self.held {
            if retired(&dependency.name) {
                continue;
            }
            match datum.latest(dependency.ecosystem.name(), &dependency.name) {
                Some(latest) => dependency.latest = Some(latest.to_string()),
                None => blind.push(format!(
                    "cannot compare {} {}: the {} datum records no answer for it",
                    dependency.ecosystem.name(),
                    dependency.name,
                    datum.version
                )),
            }
        }
        self.blind.extend(blind);
    }
}

pub fn retired(name: &str) -> bool {
    RULES.retired.iter().any(|(held, _)| held == name)
}

pub fn read(root: &Path) -> Dependencies {
    let mut found = cargo::read(
        root,
        &RULES.stable.cargo.registry,
        &RULES.stable.cargo.index,
    );
    found.normalize();
    found
}

pub fn answers(root: &Path) -> Result<Vec<plumb::datum::Answer>, String> {
    let mut found = read(root);
    found.current();
    if let Some(error) = found.blind.first() {
        return Err(error.clone());
    }
    Ok(found
        .held
        .iter()
        .filter_map(|dependency| {
            dependency
                .latest
                .as_ref()
                .map(|latest| plumb::datum::Answer {
                    ecosystem: dependency.ecosystem.name().to_string(),
                    name: dependency.name.clone(),
                    latest: latest.clone(),
                })
        })
        .collect())
}
