mod cargo;

use crate::catalog::set::RULES;
use std::path::Path;

pub use plumb_cli::{Dependency, Ecosystem};

#[derive(Default)]
pub struct Dependencies {
    pub held: Vec<Dependency>,
    pub blind: Vec<String>,
    pub missing: Option<String>,
}

impl Dependencies {
    pub(crate) fn normalize(&mut self) {
        self.held.sort();
        self.held.dedup();
        self.blind.sort();
        self.blind.dedup();
    }
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
