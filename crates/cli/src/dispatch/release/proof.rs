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
    if proof.schema != 1
        || proof.product != input.spec.product
        || proof.channel == "stable"
        || proof.commit != input.commit
        || (stable.major, stable.minor, stable.patch)
            != (candidate.major, candidate.minor, candidate.patch)
    {
        return Err("promotion seal does not prove this stable release".into());
    }
    Ok(Some(Promotion {
        seal: Box::new(proof),
        digest: sha(text.as_bytes()),
    }))
}

pub fn advance(
    current: &super::record::Pointer,
    next: &super::record::Pointer,
) -> Result<(), String> {
    if current.schema != 1
        || next.schema != 1
        || current.product != next.product
        || current.channel != "stable"
        || next.channel != "stable"
    {
        return Err("stable pointer identity mismatch".into());
    }
    let before = Version::parse(current.version.trim_start_matches('v'))
        .map_err(|error| format!("invalid current stable version: {error}"))?;
    let after = Version::parse(next.version.trim_start_matches('v'))
        .map_err(|error| format!("invalid next stable version: {error}"))?;
    if after < before {
        return Err(format!(
            "stable activation would move backwards: {} -> {}",
            current.version, next.version
        ));
    }
    Ok(())
}
