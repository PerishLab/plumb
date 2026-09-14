use crate::command::release::ReleaseMarker;
use plumb::rule::Receipt;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::command::ship) struct Workload {
    target: String,
    archive: String,
    url: String,
    receipt: Receipt,
}

fn validate(workloads: &[Workload], marker: &ReleaseMarker, image: bool) -> Result<(), String> {
    let mut held = std::collections::BTreeSet::new();
    for workload in workloads {
        if !held.insert(workload.target.as_str()) {
            return Err("duplicate binary workload target".into());
        }
        if workload.archive != marker.spec().target(&workload.target)?.archive {
            return Err("reused workload archive differs from its target".into());
        }
        if !workload.url.starts_with("https://") {
            return Err("binary workload must use an HTTPS URL".into());
        }
        super::proof::contract(marker, &workload.target)?.verify(&workload.receipt)?;
    }
    let required = marker
        .spec()
        .target
        .iter()
        .filter(|target| !image || target.triple == "x86_64-unknown-linux-gnu");
    for target in required {
        if !held.contains(target.triple.as_str()) {
            return Err(format!(
                "ship request has no proven binary workload for {}",
                target.triple
            ));
        }
    }
    if image && marker.spec().binary() && !held.contains("x86_64-unknown-linux-gnu") {
        return Err("image request has no proven Linux binary workload".into());
    }
    Ok(())
}

pub(in crate::command::ship) fn materialize(
    root: &std::path::Path,
    workloads: &[Workload],
    marker: &ReleaseMarker,
    image: bool,
) -> Result<(), String> {
    validate(workloads, marker, image)?;
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for workload in workloads {
        let target = root.join(&workload.archive);
        let status = std::process::Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--retry",
                "3",
                "--output",
            ])
            .arg(&target)
            .arg(&workload.url)
            .status()
            .map_err(|error| format!("cannot fetch {}: {error}", workload.url))?;
        if !status.success() {
            return Err(format!(
                "cannot fetch binary workload for {}",
                workload.target
            ));
        }
        workload.receipt.verify(&target)?;
        super::verify(marker.spec(), marker, &target, &workload.target)?;
    }
    Ok(())
}
