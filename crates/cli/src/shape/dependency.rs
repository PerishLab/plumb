mod cargo;
mod deno;
mod registry;

use crate::rules::RULES;
use std::path::Path;

pub use plumb_cli::{Dependency, Ecosystem};

#[derive(Default)]
pub struct Dependencies {
    pub held: Vec<Dependency>,
    pub blind: Vec<String>,
}

impl Dependencies {
    fn extend(&mut self, mut other: Self) {
        self.held.append(&mut other.held);
        self.blind.append(&mut other.blind);
    }

    fn normalize(&mut self) {
        self.held.sort();
        self.held.dedup();
        self.blind.sort();
        self.blind.dedup();
    }

    pub fn current(&mut self) {
        registry::read(self);
        self.normalize();
    }
}

pub fn read(root: &Path) -> Dependencies {
    let mut found = deno::read(root, &RULES.stable.jsr.scope);
    found.extend(cargo::read(
        root,
        &RULES.stable.cargo.registry,
        &RULES.stable.cargo.index,
    ));
    found.normalize();
    found
}
