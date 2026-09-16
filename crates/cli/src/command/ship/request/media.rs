use super::model::{Operation, Projection};
use crate::command::release::ReleaseMarker;
use crate::command::ship::adaptor;
use std::path::Path;

pub(super) fn produce(
    marker: &ReleaseMarker,
    operation: &Operation,
    input: &Path,
) -> Result<Projection, String> {
    let input = input.canonicalize().map_err(|error| error.to_string())?;
    if !input.is_dir()
        || input.join(".git").exists()
        || input
            == marker
                .spec()
                .root
                .canonicalize()
                .map_err(|error| error.to_string())?
    {
        return Err("package production requires isolated materialized source".into());
    }
    let mut spec = marker.spec().clone();
    spec.root = input.clone();
    let workload = match operation {
        Operation::Cargo => {
            let seat = tempfile::Builder::new()
                .prefix("cargo-source-")
                .tempdir()
                .map_err(|error| error.to_string())?
                .keep();
            let output = seat.join("source.tar.gz");
            super::super::archive::tree(&input, &output)?;
            output
        }
        Operation::Npm { package } => {
            super::support::pnpm(&input, true)?;
            adaptor::module::module(&spec).produce(package)?
        }
        Operation::Chart => {
            let carrier = adaptor::chart::chart(&spec);
            carrier.package("v0.0.0")?;
            let chart = spec
                .chart
                .as_ref()
                .ok_or("package production has no chart")?;
            carrier.archive(
                &adaptor::chart::name(chart)?,
                &semver::Version::new(0, 0, 0),
            )
        }
        Operation::Cfworker => {
            super::support::pnpm(&input, true)?;
            super::super::site::Worker {
                root: &input,
                version: &marker.version,
                spec: &spec,
            }
            .produce(&marker.commit)?
        }
        _ => return Err("unsupported package production operation".into()),
    };
    Ok(Projection {
        workload,
        publication: String::new(),
        receipt: None,
        depot: None,
    })
}
