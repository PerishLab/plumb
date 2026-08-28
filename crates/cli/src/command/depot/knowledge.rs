use super::{Tree, notes, product, store};
use crate::shape::depot::{self as record};
use plumb::rig::Rig;
use std::path::{Path, PathBuf};

pub struct Wanted<'a> {
    pub marker: &'a str,
    pub from: &'a str,
    pub keep: bool,
    pub dry: bool,
}

pub fn changelog(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let staged = wanted.from.is_empty();
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Changelog)?;
    let marker = crate::command::release::ReleaseMarker::at(
        root,
        &target.product,
        &target.authority,
        wanted.marker,
    )?;
    let standing = marker.digest()?;
    let source = stage(&rig, &target.product, "changelog", &wanted)?;
    let proof = crate::command::changelog::prove(root, &source, &marker.marker)?;
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(&marker.marker, false)?;
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
    if marker.commit != proof.candidate {
        return Err(format!(
            "release marker {} seals {}, but changelog proves {}",
            marker.marker, marker.commit, proof.candidate
        ));
    }
    let batch = notes::Batch::gather(&source)?;
    let plan = record::Batch::changelog(
        record::Draft {
            source: target.source.clone(),
            release: binding.release.clone(),
            timestamp: super::super::clock::mark()?,
            commit: proof.candidate.clone(),
        },
        batch.bodies,
    )?;
    let held = (|| {
        if wanted.dry {
            Ok(format!(
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
            ))
        } else {
            let advance = release.current(&binding.release)?;
            rig.depot.authority.load()?;
            store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)
        }
    })();
    confirm(root, &target, &marker, &standing)?;
    cleared(held?, staged, wanted.keep, &source)
}

pub fn skill(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let staged = wanted.from.is_empty();
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Skill)?;
    let marker = crate::command::release::ReleaseMarker::at(
        root,
        &target.product,
        &target.authority,
        wanted.marker,
    )?;
    let standing = marker.digest()?;
    let source = stage(&rig, &target.product, "skill", &wanted)?;
    let commit = Tree(root).commit()?;
    if marker.commit != commit {
        return Err(format!(
            "release marker {} seals {}, not HEAD at {commit}",
            marker.marker, marker.commit
        ));
    }
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(&marker.marker, false)?;
    let batch = notes::Batch::gather(&source)?;
    let plan = record::Batch::skill(
        record::Draft {
            source: target.source.clone(),
            release: binding.release.clone(),
            timestamp: super::super::clock::mark()?,
            commit,
        },
        batch.bodies,
    )?;
    let held = (|| {
        if wanted.dry {
            plan.manifest.encode()
        } else {
            let advance = release.current(&binding.release)?;
            rig.depot.authority.load()?;
            store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)
        }
    })();
    confirm(root, &target, &marker, &standing)?;
    cleared(held?, staged, wanted.keep, &source)
}

fn confirm(
    root: &Path,
    target: &product::Target,
    marker: &crate::command::release::ReleaseMarker,
    proof: &str,
) -> Result<(), String> {
    let after = crate::command::release::ReleaseMarker::at(
        root,
        &target.product,
        &target.authority,
        &marker.marker,
    )?;
    if after.digest()? == proof {
        Ok(())
    } else {
        Err(format!(
            "release marker {} drifted while depot was deriving knowledge",
            marker.marker
        ))
    }
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
        .join(wanted.marker))
}
