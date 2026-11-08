use super::wire::Wire;
use crate::windtun::qualification::Fault;
use crate::{guard::Runtime, packages::Package, semantics::Source};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub repository: String,
    pub marker: String,
    pub commit: String,
    pub tree: String,
}

impl Point {
    pub fn source(&self) -> Source {
        Source {
            repository: self.repository.clone(),
            commit: self.commit.clone(),
            tree: self.tree.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stable {
    #[serde(rename = "releaseVersion")]
    pub marker: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub producer: String,
    pub depot: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    pub plumb: String,
    pub ectropy: String,
    pub authority: Control,
    pub stable: BTreeMap<String, Stable>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub schema: String,
    pub source: Point,
    pub head: String,
    pub tree: String,
    pub packages: Vec<Package>,
    pub controls: Controls,
    pub domain: BTreeMap<String, String>,
    pub guard: Runtime,
}

impl World {
    pub const SCHEMA: &str = "wharf.release.snapshot/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), Fault> {
        Wire::require(
            self.schema == Self::SCHEMA,
            "unsupported effective release world",
        )?;
        let product = self
            .source
            .repository
            .strip_prefix("PerishLab/")
            .ok_or_else(|| Fault::refused("effective world source is foreign"))?;
        Wire::name(product)?;
        super::super::version::Markers::parse(&self.source.marker).map_err(Fault::refused)?;
        for identity in [
            &self.source.commit,
            &self.source.tree,
            &self.head,
            &self.tree,
        ] {
            Wire::hex(identity, 40)?;
        }
        self.guard.validate().map_err(Fault::refused)?;
        Wire::require(
            self.guard.guard.repository == self.source.repository,
            "effective world Guard names another product",
        )?;
        let tree = if self.guard.commit == self.head {
            &self.tree
        } else {
            &self.source.tree
        };
        Wire::require(
            &self.guard.guard.tree == tree,
            "effective world Guard source differs",
        )?;
        let resolution = self.guard.guard.resolution.as_ref().ok_or_else(|| {
            Fault::unknown("effective world has no actual resolved Guard input set")
        })?;
        Wire::require(
            resolution.context == "ci-latest"
                && resolution.tree == self.tree
                && resolution.packages == self.packages,
            "effective world resolution differs from actual Guard",
        )?;
        self.packages()?;
        self.controls()?;
        let names = self
            .domain
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        Wire::require(
            names == BTreeSet::from(["node.version", "pnpm.version", "rust.version"]),
            "effective world domain is incomplete or foreign",
        )?;
        for version in self.domain.values() {
            Self::stable(version)?;
        }
        Ok(())
    }

    fn packages(&self) -> Result<(), Fault> {
        Wire::require(
            self.packages.len() <= 4096,
            "effective package world exceeds bounds",
        )?;
        let mut ordered = self.packages.clone();
        ordered.sort();
        Wire::require(
            ordered == self.packages,
            "effective package world is not ordered",
        )?;
        let mut names = BTreeSet::new();
        for package in &self.packages {
            Wire::require(
                ["cargo", "npm"].contains(&package.ecosystem.as_str()),
                "effective package ecosystem is unsupported",
            )?;
            Wire::require(
                !package.name.is_empty() && package.name.len() <= 256,
                "effective package name exceeds bounds",
            )?;
            Wire::require(
                names.insert((&package.ecosystem, &package.name)),
                "effective package world repeats an identity",
            )?;
            Self::stable(&package.version)?;
        }
        Ok(())
    }

    fn controls(&self) -> Result<(), Fault> {
        let names = self
            .controls
            .stable
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        Wire::require(
            names == BTreeSet::from(["plumb", "ectropy"]),
            "effective control world is incomplete",
        )?;
        Wire::hex(&self.controls.authority.depot, 64)?;
        for (name, actual) in [
            ("plumb", &self.controls.plumb),
            ("ectropy", &self.controls.ectropy),
        ] {
            let stable = &self.controls.stable[name];
            super::super::version::Markers::stable(&stable.marker).map_err(Fault::refused)?;
            Wire::hex(&stable.commit, 40)?;
            Wire::require(
                actual == &format!("{name} {}", stable.marker),
                "effective control version differs from its stable observation",
            )?;
        }
        let plumb = &self.controls.stable["plumb"];
        Wire::require(
            self.controls.authority.producer == format!("{}@{}", plumb.marker, plumb.commit),
            "effective Plumb authority differs from its stable source",
        )?;
        Wire::require(
            self.guard.guard.plumb == self.controls.authority.producer
                && self.guard.guard.depot == self.controls.authority.depot,
            "effective Guard authority differs from its controls",
        )
    }

    fn stable(value: &str) -> Result<(), Fault> {
        let version = semver::Version::parse(value)
            .map_err(|_| Fault::refused("effective world version is invalid"))?;
        Wire::require(
            version.pre.is_empty() && version.build.is_empty(),
            "effective world requires exact stable versions",
        )
    }
}
