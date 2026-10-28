mod cargo;
mod direct;
mod npm;
pub(crate) mod topology;

use crate::catalog::set;
use std::path::Path;

pub use plumb_cli::{Dependency, Ecosystem};

#[derive(Default)]
pub struct Dependencies {
    pub held: Vec<Dependency>,
    pub blind: Vec<String>,
    pub pinned: Vec<String>,
    pub illegible: Vec<String>,
    pub missing: Option<String>,
    pub direct: Vec<(String, String)>,
    pub unread: Vec<String>,
    pub locked: bool,
}

impl Dependencies {
    pub(crate) fn normalize(&mut self) {
        self.held.sort();
        self.held.dedup();
        self.blind.sort();
        self.blind.dedup();
        self.pinned.sort();
        self.pinned.dedup();
        self.illegible.sort();
        self.illegible.dedup();
        self.direct.sort();
        self.direct.dedup();
        self.unread.sort();
        self.unread.dedup();
    }
}

pub fn read(root: &Path) -> Dependencies {
    let mut found = cargo::read(root, set::CARGO.registry, set::CARGO.index);
    (found.direct, found.unread) = direct::Workspace(root).read();
    npm::read(root, &mut found);
    found.normalize();
    found
}
