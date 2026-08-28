use super::Error;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Clone, Debug)]
pub struct Grant {
    pub version: String,
    pub url: String,
    pub sha: String,
    pointer: Option<crate::depot::v2::Pointer>,
}

pub struct Wanted<'a> {
    pub channel: &'a str,
    pub version: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Seal {
    schema: u32,
    channel: String,
    #[serde(rename = "releaseVersion")]
    version: String,
    artifacts: BTreeMap<String, Piece>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pointer {
    schema: u32,
    channel: String,
    #[serde(rename = "releaseVersion")]
    version: String,
    seal: Piece,
}

#[derive(Deserialize)]
struct Piece {
    name: String,
    url: String,
    sha256: String,
}

pub fn resolve(base: &str, channel: &str, version: Option<&str>) -> Result<Grant, Error> {
    let channel = channel.trim();
    if !valid(channel) {
        return Err(Error::Channel(channel.to_string()));
    }
    let version = version.map(str::trim);
    if channel != "stable" && version.is_none() {
        return Err(Error::Floating(channel.to_string()));
    }
    if let Some(wanted) = version
        && !belongs(channel, wanted)
    {
        return Err(Error::Version(wanted.to_string()));
    }

    let base = base.trim_end_matches('/');
    let seal = match version {
        Some(wanted) => read(&format!("{base}/v1/releases/{channel}/{wanted}/seal.json"))?,
        None => current(base)?,
    };
    if seal.schema != 1 {
        return Err(Error::Schema(seal.schema));
    }
    if seal.channel != channel {
        return Err(Error::Channel(seal.channel));
    }
    if let Some(wanted) = version
        && wanted != seal.version
    {
        return Err(Error::Version(seal.version));
    }
    if !belongs(channel, &seal.version) {
        return Err(Error::Version(seal.version));
    }
    let piece = seal.artifacts.get("skill").ok_or(Error::Absent)?;
    if !piece.name.ends_with(".tar.gz") {
        return Err(Error::Shape(piece.name.clone()));
    }
    if piece.sha256.is_empty() {
        return Err(Error::Loose);
    }
    Ok(Grant {
        version: seal.version,
        url: piece.url.clone(),
        sha: piece.sha256.clone(),
        pointer: None,
    })
}

pub fn depot(
    source: &str,
    product: &str,
    running: &str,
    wanted: Wanted<'_>,
) -> Result<Grant, Error> {
    let channel = wanted.channel.trim();
    if !valid(channel) {
        return Err(Error::Channel(channel.to_string()));
    }
    let selected = wanted.version.map(str::trim).unwrap_or(running);
    if selected != running || !belongs(channel, selected) {
        return Err(Error::Version(selected.to_string()));
    }
    let generation = crate::depot::v2::media::Generation::latest(crate::depot::v2::media::Query {
        source,
        product,
        channel,
        version: selected,
        derivative: crate::depot::v2::Kind::Skill,
    })
    .map_err(|error| Error::Fetch(source.to_string(), error))?
    .ok_or(Error::Absent)?;
    let route = crate::depot::v2::snapshots(
        &generation.pointer.release,
        generation.pointer.derivative,
        generation.mark(),
    )
    .map_err(|error| Error::Fetch(source.to_string(), error))?;
    Ok(Grant {
        version: generation.pointer.release.version.clone(),
        url: format!("{}/{route}", source.trim_end_matches('/')),
        sha: generation.digest().to_string(),
        pointer: Some(generation.pointer),
    })
}

pub fn take(grant: &Grant, name: &str) -> Result<Vec<u8>, Error> {
    let Some(pointer) = &grant.pointer else {
        return exact(&grant.url, &grant.sha);
    };
    let generation = crate::depot::v2::media::Generation::exact(&pointer.source, pointer.clone())
        .map_err(|error| Error::Fetch(grant.url.clone(), error))?;
    let mut zip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    {
        let mut archive = tar::Builder::new(&mut zip);
        for object in &generation.manifest.objects {
            let bytes = generation
                .read(&object.path)
                .map_err(|error| Error::Fetch(grant.url.clone(), error))?;
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, format!("{name}/{}", object.path), &bytes[..])
                .map_err(|error| Error::Unpack(error.to_string()))?;
        }
        archive
            .finish()
            .map_err(|error| Error::Unpack(error.to_string()))?;
    }
    zip.flush()
        .map_err(|error| Error::Unpack(error.to_string()))?;
    zip.finish()
        .map_err(|error| Error::Unpack(error.to_string()))
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

fn current(base: &str) -> Result<Seal, Error> {
    let pointer: Pointer = read(&format!("{base}/v1/channels/stable.json"))?;
    if pointer.schema != 1 || pointer.channel != "stable" {
        return Err(Error::Channel(pointer.channel));
    }
    let body = exact(&pointer.seal.url, &pointer.seal.sha256)?;
    let seal: Seal =
        serde_json::from_slice(&body).map_err(|error| Error::Parse(error.to_string()))?;
    if seal.version != pointer.version {
        return Err(Error::Version(seal.version));
    }
    Ok(seal)
}

fn read<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, Error> {
    let body = draw(url)?;
    serde_json::from_slice(&body).map_err(|error| Error::Parse(error.to_string()))
}

fn exact(url: &str, sha: &str) -> Result<Vec<u8>, Error> {
    if sha.is_empty() {
        return Err(Error::Loose);
    }
    let bytes = draw(url)?;
    let seen = stamp(&bytes);
    if seen != sha {
        return Err(Error::Digest(sha.to_string(), seen));
    }
    Ok(bytes)
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

fn valid(channel: &str) -> bool {
    let mut bytes = channel.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn belongs(channel: &str, release: &str) -> bool {
    let Some(raw) = release.strip_prefix('v') else {
        return false;
    };
    let Ok(version) = semver::Version::parse(raw) else {
        return false;
    };
    if channel == "stable" {
        return version.pre.is_empty();
    }
    let mut parts = version.pre.as_str().split('.');
    parts.next() == Some(channel)
        && parts
            .next()
            .is_some_and(|number| number.parse::<u64>().is_ok_and(|number| number > 0))
        && parts.next().is_none()
}
