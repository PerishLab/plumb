use super::project::{Scope, Source, fetch};
use super::projection::{self, Request};
use crate::command::ship::adaptor::chart::{Chart, name, release, stamped};
use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn run(
    carrier: &Chart<'_>,
    version: &str,
    credential: &str,
    reuse: &str,
) -> Result<String, String> {
    let chart = carrier
        .spec
        .chart
        .as_ref()
        .ok_or_else(|| format!("{} has no chart attachment", carrier.spec.product))?;
    let semver = release(version)?;
    let held = name(chart)?;
    let source = Source::parse(reuse)?;
    let archive = match source.kind.as_str() {
        "none" => {
            let scope = Scope::read(&carrier.seat(&held).join("Chart.yaml"))?;
            carrier.package(version)?;
            scope.restore()?;
            carrier.archive(&held, &semver)
        }
        "workload" => {
            let bytes = fetch("chart", &source.source)?;
            let archive = carrier.archive(&held, &semver);
            if let Some(parent) = archive.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot open {}: {error}", parent.display()))?;
            }
            restamp(&bytes, &archive, &held, &semver)?;
            archive
        }
        "url" => return Err("a held publication URL must skip the chart action".into()),
        _ => unreachable!(),
    };
    let publication = projection::run(
        carrier,
        Request {
            chart,
            version: &semver,
            credential,
            archive: &archive,
        },
    )?;
    serde_json::to_string(&serde_json::json!({
        "format": "plumb.chart-project/v1",
        "chart": chart.chart,
        "version": version,
        "sha256": crate::command::release::record::digest(&archive)?.0,
        "workload": archive,
        "publication": publication,
    }))
    .map_err(|error| format!("cannot encode chart project: {error}"))
}

fn restamp(bytes: &[u8], output: &Path, chart: &str, version: &Version) -> Result<(), String> {
    let manifest = PathBuf::from(chart).join("Chart.yaml");
    if current(bytes, &manifest, version)? {
        return std::fs::write(output, bytes)
            .map_err(|error| format!("cannot create {}: {error}", output.display()));
    }
    let mut source = tar::Archive::new(GzDecoder::new(bytes));
    let file = File::create(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    let gzip = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut target = tar::Builder::new(gzip);
    let mut found = false;
    for entry in source
        .entries()
        .map_err(|error| format!("cannot read reusable chart workload: {error}"))?
    {
        let mut entry =
            entry.map_err(|error| format!("cannot read reusable chart workload: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("cannot read reusable chart path: {error}"))?
            .into_owned();
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|error| format!("cannot read reusable chart entry: {error}"))?;
        let mut header = entry.header().clone();
        if path == manifest {
            let text = String::from_utf8(body)
                .map_err(|error| format!("cannot parse reusable Chart.yaml: {error}"))?;
            body = stamped(&text, version).into_bytes();
            header.set_size(body.len() as u64);
            found = true;
        }
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_cksum();
        target
            .append(&header, body.as_slice())
            .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    }
    if !found {
        return Err(format!("reusable workload carries no {chart}/Chart.yaml"));
    }
    let gzip = target
        .into_inner()
        .map_err(|error| format!("cannot finish {}: {error}", output.display()))?;
    gzip.finish()
        .map_err(|error| format!("cannot finish {}: {error}", output.display()))?;
    Ok(())
}

fn current(bytes: &[u8], manifest: &Path, version: &Version) -> Result<bool, String> {
    let mut source = tar::Archive::new(GzDecoder::new(bytes));
    for entry in source
        .entries()
        .map_err(|error| format!("cannot read reusable chart workload: {error}"))?
    {
        let mut entry =
            entry.map_err(|error| format!("cannot read reusable chart workload: {error}"))?;
        if entry
            .path()
            .map_err(|error| format!("cannot read reusable chart path: {error}"))?
            != manifest
        {
            continue;
        }
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|error| format!("cannot parse reusable Chart.yaml: {error}"))?;
        let expected = version.to_string();
        let chart = text
            .lines()
            .any(|line| value(line, "version:") == Some(expected.as_str()));
        let app = text
            .lines()
            .any(|line| value(line, "appVersion:") == Some(expected.as_str()));
        return Ok(chart && app);
    }
    Ok(false)
}

fn value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)
        .map(str::trim)
        .map(|held| held.trim_matches('"'))
}
