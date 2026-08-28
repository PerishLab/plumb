use super::{Tree, notes, product, store};
use crate::shape::depot::{self as record};
use plumb::rig::Rig;
use std::path::{Path, PathBuf};

pub struct Wanted<'a> {
    pub version: &'a str,
    pub from: &'a str,
    pub keep: bool,
    pub dry: bool,
}

pub fn changelog(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let staged = wanted.from.is_empty();
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Changelog)?;
    let source = stage(&rig, &target.product, "changelog", &wanted)?;
    let proof = crate::command::changelog::prove(root, &source, wanted.version)?;
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(wanted.version, false)?;
    if binding.release.channel != "stable" {
        return Err(format!(
            "the {} channel does not owe a changelog derivative",
            binding.release.channel
        ));
    }
    if binding.release.commit != proof.candidate {
        return Err(format!(
            "changelog proves commit {}, but release {} seals {}",
            proof.candidate, binding.release.version, binding.release.commit
        ));
    }
    let batch = notes::Batch::gather(&source)?;
    let plan = record::Batch::changelog(
        record::Draft {
            source: target.source,
            release: binding.release.clone(),
            timestamp: super::super::clock::mark()?,
            commit: proof.candidate.clone(),
        },
        batch.bodies,
    )?;
    if wanted.dry {
        return Ok(format!(
            "{}\n{} lines within a budget of {} for {} units",
            plan.manifest.encode()?,
            proof
                .languages
                .values()
                .map(|held| held.lines)
                .max()
                .unwrap_or_default(),
            proof
                .languages
                .values()
                .map(|held| held.budget)
                .max()
                .unwrap_or_default(),
            proof.units
        ));
    }
    let advance = release.current(&binding.release)?;
    rig.depot.authority.load()?;
    let held = store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)?;
    cleared(held, staged, wanted.keep, &source)
}

pub fn skill(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let staged = wanted.from.is_empty();
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Skill)?;
    let source = stage(&rig, &target.product, "skill", &wanted)?;
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(wanted.version, false)?;
    let batch = notes::Batch::gather(&source)?;
    let plan = record::Batch::skill(
        record::Draft {
            source: target.source,
            release: binding.release.clone(),
            timestamp: super::super::clock::mark()?,
            commit: Tree(root).commit()?,
        },
        batch.bodies,
    )?;
    if wanted.dry {
        return plan.manifest.encode();
    }
    let advance = release.current(&binding.release)?;
    rig.depot.authority.load()?;
    let held = store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)?;
    cleared(held, staged, wanted.keep, &source)
}

fn cleared(held: String, staged: bool, keep: bool, source: &Path) -> Result<String, String> {
    if staged && !keep {
        std::fs::remove_dir_all(source)
            .map_err(|error| format!("cannot clear {}: {error}", source.display()))?;
        return Ok(format!("{held}, and cleared {}", source.display()));
    }
    Ok(held)
}

fn stage(
    rig: &Rig,
    product: &str,
    derivative: &str,
    wanted: &Wanted<'_>,
) -> Result<PathBuf, String> {
    if !wanted.from.is_empty() {
        return Ok(PathBuf::from(wanted.from));
    }
    if rig.home.is_empty() {
        return Err("no data home; set PLUMB_HOME".into());
    }
    Ok(PathBuf::from(&rig.home)
        .join("depot")
        .join("stage")
        .join(product)
        .join(derivative)
        .join(wanted.version))
}
