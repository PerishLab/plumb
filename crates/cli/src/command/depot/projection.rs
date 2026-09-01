use super::configuration::Tree;
use plumb::rig::Rig;
use serde::Deserialize;

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
    crate::command::release::knowledge(&spec.product, &spec.authority)
        .binding(&marker.marker, spec.binary())?;
    rig.activate.load()?;
    let result = match projection {
        Kind::Channel => {
            crate::command::release::projection::channel(&spec, &marker, &rig.activate)
        }
        Kind::Managers => {
            crate::command::release::projection::managers(&spec, &marker, &rig.activate)
        }
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
