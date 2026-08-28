use crate::command::ship::adaptor::chart::{Chart, name, owner};
use semver::Version;
use std::path::Path;

pub struct Request<'a> {
    pub chart: &'a crate::shape::release::Chart,
    pub version: &'a Version,
    pub credential: &'a str,
    pub archive: &'a Path,
}

pub fn run(carrier: &Chart<'_>, request: Request<'_>) -> Result<String, String> {
    let identity = crate::command::ship::attachment::Identity {
        user: &request.chart.account,
        token: crate::command::ship::attachment::credential(request.credential)?,
    };
    let held = name(request.chart)?;
    let owner = owner(request.chart)?;
    carrier.login(&request.chart.registry, &identity)?;
    let seat = format!("oci://{}/{owner}/{held}", request.chart.registry);
    if let Some(carried) = carrier.carried(&seat, request.version)? {
        same(&seat, request.archive, &carried)?;
        return publication(request.chart, request.version);
    }
    carrier.helm([
        "push",
        &request.archive.to_string_lossy(),
        &format!("oci://{}/{owner}", request.chart.registry),
    ])?;
    carrier.helm([
        "show",
        "chart",
        &seat,
        "--version",
        &request.version.to_string(),
    ])?;
    let carried = carrier
        .carried(&seat, request.version)?
        .ok_or_else(|| format!("{seat} reports no archive after publishing it"))?;
    same(&seat, request.archive, &carried)?;
    publication(request.chart, request.version)
}

fn publication(chart: &crate::shape::release::Chart, version: &Version) -> Result<String, String> {
    Ok(format!(
        "https://{}/{}/-/packages/container/{}/{}",
        chart
            .registry
            .trim_start_matches("https://")
            .trim_end_matches('/'),
        owner(chart)?,
        name(chart)?,
        version
    ))
}

fn same(seat: &str, archive: &Path, carried: &str) -> Result<(), String> {
    let held = crate::command::release::record::digest(archive)?.0;
    if carried == held {
        Ok(())
    } else {
        Err(format!(
            "published chart drift: {seat} holds {carried} while this projection carries {held}"
        ))
    }
}
