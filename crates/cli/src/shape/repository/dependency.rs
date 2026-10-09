mod cargo;
mod direct;

use crate::catalog::set;
use std::path::Path;

pub use plumb_cli::{Dependency, Ecosystem};

#[derive(Default)]
pub struct Dependencies {
    pub held: Vec<Dependency>,
    pub blind: Vec<String>,
    pub missing: Option<String>,
    pub direct: Vec<(String, String)>,
    pub unread: Vec<String>,
}

impl Dependencies {
    pub(crate) fn normalize(&mut self) {
        self.held.sort();
        self.held.dedup();
        self.blind.sort();
        self.blind.dedup();
        self.direct.sort();
        self.direct.dedup();
        self.unread.sort();
        self.unread.dedup();
    }
}

pub fn read(root: &Path) -> Dependencies {
    let mut found = cargo::read(root, set::CARGO.registry, set::CARGO.index);
    (found.direct, found.unread) = direct::Workspace(root).read();
    found.normalize();
    found
}
