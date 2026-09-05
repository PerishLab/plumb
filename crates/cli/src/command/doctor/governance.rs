use crate::catalog::rules::depot::DEPOT_SCHEMA;
use crate::command::precommit::tree::Index;
use crate::judge::finding::{Finding, Seed};
use crate::shape::product::{self, Profile, Source};
use std::path::Path;

pub struct Read {
    pub configuration: Option<String>,
    pub profile: Option<String>,
    pub rooted: bool,
    pub view: Option<Index>,
    pub findings: Vec<Finding>,
}

pub fn inspect(root: &Path) -> Read {
    let mut read = Read {
        configuration: None,
        profile: None,
        rooted: false,
        view: None,
        findings: Vec::new(),
    };
    match product::governance(root).map(|target| target.and_then(|held| held.profile)) {
        Ok(Some(profile)) => read.profile(root, profile),
        Ok(None) => {}
        Err(error) => read.blind(error),
    }
    read
}

impl Read {
    fn profile(&mut self, root: &Path, profile: Profile) {
        self.configuration = Some(profile.configuration.clone());
        self.profile = Some(profile.digest.clone());
        match profile.source {
            Source::Repository => {
                self.rooted = true;
                self.exact(root, &profile);
            }
            Source::Depot => self.project(root, &profile),
        }
    }

    fn exact(&mut self, root: &Path, profile: &Profile) {
        for (name, expected) in [
            ("plumb.toml", profile.manifest.as_str()),
            ("ectropy.toml", profile.ectropy.as_str()),
        ] {
            match std::fs::read_to_string(root.join(name)) {
                Ok(actual) if actual == expected => {}
                Ok(_) => self.wrong(format!("{name} differs from its exact Depot profile")),
                Err(error) => self.wrong(format!(
                    "{name} is required by its repository migration state: {error}"
                )),
            }
        }
    }

    fn project(&mut self, root: &Path, profile: &Profile) {
        if root.join("plumb.toml").exists() || root.join("ectropy.toml").exists() {
            self.wrong("a Depot-governed product must not carry plumb.toml or ectropy.toml");
        }
        match Index::working(root).and_then(|index| index.govern(profile).map(|()| index)) {
            Ok(index) => self.view = Some(index),
            Err(error) => self.blind(error),
        }
    }

    fn wrong(&mut self, evidence: impl Into<String>) {
        self.findings
            .push(Finding::new(Seed::wrong(&DEPOT_SCHEMA, evidence.into())));
    }

    fn blind(&mut self, evidence: impl Into<String>) {
        self.findings
            .push(Finding::new(Seed::blind(&DEPOT_SCHEMA, evidence.into())));
    }
}
