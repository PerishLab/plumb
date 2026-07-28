use super::Error;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct Grant {
    pub version: String,
    pub url: String,
    pub sha: String,
}

#[derive(Deserialize)]
struct Release {
    #[serde(rename = "releaseVersion")]
    version: String,
    artifacts: Bundle,
}

#[derive(Deserialize)]
struct Bundle {
    #[serde(rename = "skillTarGz")]
    skill: Option<Piece>,
}

#[derive(Deserialize)]
struct Piece {
    name: String,
    url: String,
    sha256: Option<String>,
}

pub fn resolve(base: &str, channel: &str, version: Option<&str>) -> Result<Grant, Error> {
    let base = base.trim_end_matches('/');
    let url = match version {
        Some(version) => format!("{base}/{channel}/versions/{}/metadata.json", tidy(version)),
        None => format!("{base}/{channel}/latest/metadata.json"),
    };
    let body = draw(&url)?;
    let release: Release =
        serde_json::from_slice(&body).map_err(|error| Error::Parse(error.to_string()))?;
    let piece = release.artifacts.skill.ok_or(Error::Absent)?;
    if !piece.name.ends_with(".tar.gz") {
        return Err(Error::Shape(piece.name));
    }
    Ok(Grant {
        version: release.version,
        url: piece.url,
        sha: piece.sha256.unwrap_or_default(),
    })
}

pub fn take(grant: &Grant) -> Result<Vec<u8>, Error> {
    let bytes = draw(&grant.url)?;
    if grant.sha.is_empty() {
        return Err(Error::Loose);
    }
    let seen = stamp(&bytes);
    if seen != grant.sha {
        return Err(Error::Digest(grant.sha.clone(), seen));
    }
    Ok(bytes)
}

pub fn stamp(bytes: &[u8]) -> String {
    let mut sponge = Sha256::new();
    sponge.update(bytes);
    sponge
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn draw(url: &str) -> Result<Vec<u8>, Error> {
    let mut body = ureq::get(url)
        .call()
        .map_err(|error| Error::Fetch(url.to_string(), error.to_string()))?
        .into_body();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut body.as_reader(), &mut bytes)
        .map_err(|error| Error::Fetch(url.to_string(), error.to_string()))?;
    Ok(bytes)
}

fn tidy(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}
