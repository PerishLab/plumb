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
    let marker = crate::command::release::ReleaseMarker::bound(root, raw)?;
    let spec = marker.spec();
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
    let result = crate::command::ship::site::deploy(root, spec, &request.worker, &request.version)?;
    let after = crate::command::release::ReleaseMarker::bound(root, &marker.marker)?;
    if after.digest()? != proof {
        return Err(format!(
            "release marker {} drifted while depot deployed its worker",
            marker.marker
        ));
    }
    Ok(result)
}

pub fn project(raw: &str, projection: Kind) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let marker = crate::command::release::ReleaseMarker::bound(&rig.release.root, raw)?;
    let spec = marker.spec();
    let proof = marker.digest()?;
    crate::command::release::knowledge(&spec.product, &spec.authority)
        .binding(&marker.marker, spec.binary())?;
    let bucket = format!("perish-{}-releases", spec.product);
    super::authority::project(rig.activate, &bucket, &proof, |authority| {
        let fresh =
            crate::command::release::ReleaseMarker::bound(&rig.release.root, &marker.marker)?;
        if fresh.digest()? != proof {
            return Err("release marker drifted before depot projection".into());
        }
        let result = match projection {
            Kind::Channel => crate::command::release::projection::channel(spec, &marker, authority),
            Kind::Managers => {
                crate::command::release::projection::managers(spec, &marker, authority)
            }
        }?;
        let after =
            crate::command::release::ReleaseMarker::bound(&rig.release.root, &marker.marker)?;
        if after.digest()? != proof {
            return Err(format!(
                "release marker {} drifted while depot projected it",
                marker.marker
            ));
        }
        Ok(result)
    })
}
