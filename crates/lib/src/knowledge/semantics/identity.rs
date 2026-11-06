use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::version::Markers;
use super::{Coverage, Decision, Kind, Material, Review, wire::Wire};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub repository: String,
    pub commit: String,
    pub tree: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub product: String,
    pub source: Source,
    pub materials: Vec<Material>,
    pub coverage: Vec<Coverage>,
}

impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        Wire::name(&self.product)?;
        if self.source.repository != format!("PerishLab/{}", self.product) {
            return Err("snapshot source does not belong to its product".into());
        }
        Wire::hex(&self.source.commit, 40)?;
        Wire::hex(&self.source.tree, 40)?;
        if !(1..=4096).contains(&self.materials.len()) || !(1..=4096).contains(&self.coverage.len())
        {
            return Err("snapshot requires bounded nonempty materials and coverage".into());
        }
        self.inventory()
    }

    fn inventory(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        let mut cases = BTreeSet::new();
        let mut schema = false;
        for material in &self.materials {
            material.validate()?;
            if !names.insert((material.kind, &material.name)) {
                return Err("snapshot repeats a material identity".into());
            }
            if material.kind == Kind::Case {
                cases.insert(&material.name);
            }
            schema |= material.kind == Kind::Schema;
        }
        let mut coverage = BTreeSet::new();
        let mut covered = BTreeSet::new();
        for item in &self.coverage {
            item.validate()?;
            if !coverage.insert(item) {
                return Err("snapshot repeats a coverage identity".into());
            }
            covered.insert(&item.case);
        }
        if !schema || cases.is_empty() || cases != covered {
            return Err("snapshot requires schema and exactly covered case materials".into());
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate()?;
        let mut held = self.clone();
        held.materials.sort();
        held.coverage.sort();
        serde_json::to_vec(&held)
            .map(|bytes| Wire::digest(&bytes))
            .map_err(|error| format!("cannot encode release snapshot: {error}"))
    }

    pub(super) fn scope(&self) -> BTreeSet<&Coverage> {
        self.coverage.iter().collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub marker: String,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub stable: Release,
    pub anchor: Release,
    pub review: Option<Review>,
}

impl Baseline {
    pub fn validate(&self) -> Result<(), String> {
        let stable = Markers::stable(&self.stable.marker)?;
        let anchor = Markers::stable(&self.anchor.marker)?;
        self.stable.snapshot.validate()?;
        self.anchor.snapshot.validate()?;
        if (stable.major, stable.minor) != (anchor.major, anchor.minor)
            || stable < anchor
            || self.stable.snapshot.product != self.anchor.snapshot.product
        {
            return Err("stable and anchor must belong to one ordered product family".into());
        }
        let same = self.stable.snapshot.digest()? == self.anchor.snapshot.digest()?;
        if self.stable.snapshot.scope() != self.anchor.snapshot.scope() {
            return Err("stable coverage has drifted from its X.Y anchor".into());
        }
        if self.stable.marker == self.anchor.marker && !same {
            return Err("one release marker cannot name different boundary snapshots".into());
        }
        match &self.review {
            Some(review) => {
                review.bind(
                    &self.anchor.snapshot,
                    &self.stable.snapshot,
                    &self.anchor.snapshot,
                )?;
                if review.decision != Decision::Same {
                    return Err("a Z family requires an unchanged anchor boundary".into());
                }
            }
            None if !same => return Err("stable snapshot has no anchor continuity review".into()),
            None => {}
        }
        Ok(())
    }
}
