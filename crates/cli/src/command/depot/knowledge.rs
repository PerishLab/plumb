use super::{configuration::Tree, product, store};
use plumb::rig::Rig;
use std::path::Path;

pub struct Wanted<'a> {
    pub marker: &'a str,
    pub from: &'a str,
    pub dry: bool,
}

pub fn changelog(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Changelog)?;
    let marker = crate::command::release::ReleaseMarker::at(
        root,
        &target.product,
        &target.authority,
        wanted.marker,
    )?;
    let standing = marker.digest()?;
    let source = source(wanted.from)?;
    let proof = crate::command::changelog::prove(root, source, &marker.marker)?;
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(&marker.marker, false)?;
    if binding.release.commit != marker.commit || binding.release.channel != marker.channel {
        return Err(format!(
            "release marker {} does not bind its published release seal",
            marker.marker
        ));
    }
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
    let bundle = plumb::depot::v3::Bundle::read(
        source,
        identity(
            &target,
            &marker,
            &standing,
            plumb::depot::v3::Kind::Changelog,
        ),
    )?;
    let held = (|| {
        if wanted.dry {
            let manifest = String::from_utf8(bundle.manifest.encode()?)
                .map_err(|error| format!("depot manifest is not UTF-8: {error}"))?;
            Ok(format!(
                "{}\n{} lines within a budget of {} for {} units",
                manifest,
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
            rig.depot.authority.load()?;
            store::Remote::new(&rig.depot.authority)?.publish(
                &bundle,
                &target.source,
                super::super::clock::ahead(0)?,
            )
        }
    })();
    confirm(root, &target, &marker, &standing)?;
    held
}

pub fn skill(root: &Path, wanted: Wanted<'_>) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let target = product::resolve(root, &rig, plumb::depot::v2::Kind::Skill)?;
    let marker = crate::command::release::ReleaseMarker::at(
        root,
        &target.product,
        &target.authority,
        wanted.marker,
    )?;
    let standing = marker.digest()?;
    let source = source(wanted.from)?;
    let commit = Tree(root).commit()?;
    if marker.commit != commit {
        return Err(format!(
            "release marker {} seals {}, not HEAD at {commit}",
            marker.marker, marker.commit
        ));
    }
    let release = crate::command::release::knowledge(&target.product, &target.authority);
    let binding = release.binding(&marker.marker, false)?;
    if binding.release.commit != marker.commit || binding.release.channel != marker.channel {
        return Err(format!(
            "release marker {} does not bind its published release seal",
            marker.marker
        ));
    }
    let bundle = plumb::depot::v3::Bundle::read(
        source,
        identity(&target, &marker, &standing, plumb::depot::v3::Kind::Skill),
    )?;
    if !bundle.bodies.contains_key("SKILL.md") {
        return Err("skill generation holds no SKILL.md".into());
    }
    let held = (|| {
        if wanted.dry {
            String::from_utf8(bundle.manifest.encode()?)
                .map_err(|error| format!("depot manifest is not UTF-8: {error}"))
        } else {
            rig.depot.authority.load()?;
            store::Remote::new(&rig.depot.authority)?.publish(
                &bundle,
                &target.source,
                super::super::clock::ahead(0)?,
            )
        }
    })();
    confirm(root, &target, &marker, &standing)?;
    held
}

fn source(raw: &str) -> Result<&Path, String> {
    if raw.is_empty() {
        Err("depot knowledge publication requires an explicit --from directory".into())
    } else {
        Ok(Path::new(raw))
    }
}

fn identity(
    target: &product::Target,
    marker: &crate::command::release::ReleaseMarker,
    digest: &str,
    kind: plumb::depot::v3::Kind,
) -> plumb::depot::v3::Identity {
    plumb::depot::v3::Identity {
        product: target.product.clone(),
        channel: marker.channel.clone(),
        version: marker.marker.clone(),
        marker: plumb::depot::v3::Marker {
            name: marker.marker.clone(),
            sha256: digest.to_string(),
        },
        kind,
    }
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
