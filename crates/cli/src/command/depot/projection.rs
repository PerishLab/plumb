use super::configuration::Tree;
use plumb::rig::Rig;
use serde::Deserialize;
use std::path::Path;

pub enum Kind {
    Channel,
    Managers,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Worker {
    schema: String,
    marker: String,
    worker: String,
    version: String,
}

pub fn worker(raw: &str, request: &str) -> Result<String, String> {
    let request: Worker = serde_json::from_str(request)
        .map_err(|error| format!("cannot parse worker projection: {error}"))?;
    if request.schema != "plumb.depot-worker/v1" {
        return Err("worker projection schema must be plumb.depot-worker/v1".into());
    }
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let root = &rig.release.root;
    let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
    let marker =
        crate::command::release::ReleaseMarker::at(root, &spec.product, &spec.authority, raw)?;
    if request.marker != marker.marker {
        return Err(format!(
            "worker projection belongs to {}, not release marker {}",
            request.marker, marker.marker
        ));
    }
    let proof = marker.digest()?;
    let head = Tree(root).commit()?;
    if head != marker.commit {
        return Err(format!(
            "release marker {} seals {}, not HEAD at {head}",
            marker.marker, marker.commit
        ));
    }
    let result = crate::command::ship::site::deploy(root, &request.worker, &request.version)?;
    let after = crate::command::release::ReleaseMarker::at(
        root,
        &spec.product,
        &spec.authority,
        &marker.marker,
    )?;
    if after.digest()? != proof {
        return Err(format!(
            "release marker {} drifted while depot deployed its worker",
            marker.marker
        ));
    }
    Ok(result)
}

pub fn project(raw: &str, projection: Kind) -> Result<String, String> {
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let marker = crate::command::release::ReleaseMarker::at(
        &rig.release.root,
        &spec.product,
        &spec.authority,
        raw,
    )?;
    let proof = marker.digest()?;
    let capsule = crate::command::release::capsule(&rig.release)?;
    bound(&capsule, &marker)?;
    rig.activate.load()?;
    let result = match projection {
        Kind::Channel => {
            let configuration = spec
                .depot
                .as_ref()
                .is_some_and(|depot| {
                    depot
                        .derivatives
                        .contains(&plumb::depot::v2::Kind::Configuration)
                })
                .then(|| Tree(&rig.release.root).advance(raw))
                .transpose()?;
            let release = crate::command::release::storage::activate(&capsule, &rig.activate)?;
            Ok(configuration.map_or(release.clone(), |configuration| {
                format!("{configuration}\n{release}")
            }))
        }
        Kind::Managers => crate::command::release::storage::shift(&capsule, &rig.activate),
    }?;
    let after = crate::command::release::ReleaseMarker::at(
        &rig.release.root,
        &spec.product,
        &spec.authority,
        &marker.marker,
    )?;
    if after.digest()? != proof {
        return Err(format!(
            "release marker {} drifted while depot projected it",
            marker.marker
        ));
    }
    Ok(result)
}

fn bound(path: &Path, marker: &crate::command::release::ReleaseMarker) -> Result<(), String> {
    let (capsule, root) = crate::command::release::record::Capsule::read(path)?;
    let text = std::fs::read_to_string(root.join(&capsule.seal.source))
        .map_err(|error| format!("cannot read capsule seal: {error}"))?;
    let seal: crate::command::release::record::Seal = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse capsule seal: {error}"))?;
    let standing = (
        capsule.product.as_str(),
        capsule.authority.as_str(),
        capsule.channel.as_str(),
        capsule.version.as_str(),
        seal.commit.as_str(),
    );
    let wanted = (
        marker.product.as_str(),
        marker.authority.as_str(),
        marker.channel.as_str(),
        marker.marker.as_str(),
        marker.commit.as_str(),
    );
    if standing == wanted {
        Ok(())
    } else {
        Err(format!(
            "capsule does not bind release marker {} at {}",
            marker.marker, marker.commit
        ))
    }
}
