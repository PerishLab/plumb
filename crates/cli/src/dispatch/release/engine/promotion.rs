use super::super::{manager, model::Spec, proof, verify};
use plumb::forgejo::git;
use semver::Version;
use std::path::Path;
use std::process::Command;

pub struct Exact {
    pub channel: String,
    pub version: String,
}

pub fn derive(spec: &Spec, commit: &str, version: &str) -> Result<Exact, String> {
    proof::commit(commit)?;
    manager::intent("stable", version)?;
    let wanted = trunk(version)?;
    let mut found = Vec::new();
    for tag in git::tags(&spec.root, commit)? {
        let Ok(channel) = manager::channel(&tag) else {
            continue;
        };
        if channel == "stable" || trunk(&tag)? != wanted {
            continue;
        }
        if verify::optional(&url(spec, &channel, &tag))?.is_some() {
            found.push(Exact {
                channel,
                version: tag,
            });
        }
    }
    settle(found, commit, version)
}

pub fn fetch(spec: &Spec, commit: &str, version: &str, output: &Path) -> Result<String, String> {
    let exact = derive(spec, commit, version)?;
    if output.exists() {
        return Err(format!(
            "promotion proof already exists: {}",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let url = url(spec, &exact.channel, &exact.version);
    verify::inspect(&url, false)?;
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(output)
        .arg(&url)
        .status()
        .map_err(|error| format!("cannot fetch promotion proof: {error}"))?;
    if !status.success() {
        return Err(format!("cannot fetch promotion proof from {url}"));
    }
    Ok(format!(
        "fetched promotion proof {} {}",
        exact.channel, exact.version
    ))
}

fn settle(found: Vec<Exact>, commit: &str, version: &str) -> Result<Exact, String> {
    if found.len() == 1 {
        return found
            .into_iter()
            .next()
            .ok_or_else(|| "promotion source vanished".to_string());
    }
    if found.is_empty() {
        return Err(format!(
            "no published exact seal stands at {commit} for stable {version}"
        ));
    }
    let named = found
        .iter()
        .map(|exact| exact.version.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "{} published exact seals stand at {commit} for stable {version}: {named}; \
         rerun one exact release instead of tagging another",
        found.len()
    ))
}

fn trunk(version: &str) -> Result<(u64, u64, u64), String> {
    let parsed = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid release version: {error}"))?;
    Ok((parsed.major, parsed.minor, parsed.patch))
}

fn url(spec: &Spec, channel: &str, version: &str) -> String {
    format!(
        "{}/v1/releases/{channel}/{version}/seal.json",
        spec.authority
    )
}
