use super::model::Spec;
use super::record::{Promotion, Seal, sha};
use semver::Version;
use std::path::Path;

pub struct Claim<'a> {
    pub spec: &'a Spec,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub path: Option<&'a Path>,
}

pub fn commit(value: &str) -> Result<(), String> {
    if !(40..=64).contains(&value.len())
        || !value
            .chars()
            .all(|held| held.is_ascii_digit() || ('a'..='f').contains(&held))
    {
        return Err(format!("invalid release commit: {value}"));
    }
    Ok(())
}

pub fn promotion(input: Claim<'_>) -> Result<Option<Promotion>, String> {
    if input.channel != "stable" {
        if input.path.is_some() {
            return Err("only stable accepts a promotion proof".into());
        }
        return Ok(None);
    }
    let path = input
        .path
        .ok_or_else(|| "stable requires an exact promotion seal".to_string())?;
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read promotion seal {}: {error}", path.display()))?;
    let proof: Seal = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse promotion seal {}: {error}", path.display()))?;
    let stable =
        Version::parse(input.version.trim_start_matches('v')).map_err(|error| error.to_string())?;
    let candidate =
        Version::parse(proof.version.trim_start_matches('v')).map_err(|error| error.to_string())?;
    let release = (stable.major, stable.minor, stable.patch);
    let source = (candidate.major, candidate.minor, candidate.patch);
    let current = proof.schema == 1 && proof.channel != "stable";
    let identity = proof.product == input.spec.product && proof.commit == input.commit;
    if !current || !identity || release != source {
        return Err("promotion seal does not prove this stable release".into());
    }
    super::generator::audit(&proof)?;
    Ok(Some(Promotion {
        seal: Box::new(proof),
        digest: sha(text.as_bytes()),
    }))
}

pub fn advance(
    current: &super::record::Pointer,
    next: &super::record::Pointer,
) -> Result<(), String> {
    let schema = current.schema == 1 && next.schema == 1;
    let held = current.channel == next.channel;
    if !schema || !held || current.product != next.product {
        return Err(format!(
            "{} pointer identity mismatch",
            next.channel.as_str()
        ));
    }
    let before = Version::parse(current.version.trim_start_matches('v'))
        .map_err(|error| format!("invalid current {} version: {error}", current.channel))?;
    let after = Version::parse(next.version.trim_start_matches('v'))
        .map_err(|error| format!("invalid next {} version: {error}", next.channel))?;
    if after < before {
        return Err(format!(
            "{} activation would move backwards: {} -> {}",
            next.channel, current.version, next.version
        ));
    }
    Ok(())
}
