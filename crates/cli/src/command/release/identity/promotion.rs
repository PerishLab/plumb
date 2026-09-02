use super::super::channel;
use super::super::truth::{proof, record, verify};
use crate::shape::release::Spec;
use plumb::forgejo::git;
use semver::Version;
use std::path::Path;
use std::process::Command;

pub struct Exact {
    pub channel: String,
    pub version: String,
}

pub struct Promotion<'a> {
    spec: &'a Spec,
}

impl<'a> Promotion<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Self { spec }
    }

    pub fn derive(&self, commit: &str, version: &str) -> Result<Exact, String> {
        derive(&self.spec.root, &self.spec.authority, commit, version)
    }

    pub fn fetch(&self, commit: &str, version: &str, output: &Path) -> Result<String, String> {
        let exact = self.derive(commit, version)?;
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
        let url = self.url(&exact.channel, &exact.version);
        verify::inspect(&url, false)?;
        let status = Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--retry",
                "3",
                "--retry-all-errors",
                "--retry-delay",
                "1",
                "--output",
            ])
            .arg(output)
            .arg(&url)
            .status()
            .map_err(|error| format!("cannot fetch promotion proof: {error}"))?;
        if !status.success() {
            let _ = std::fs::remove_file(output);
            return Err(format!("cannot fetch promotion proof from {url}"));
        }
        let text = match std::fs::read_to_string(output) {
            Ok(text) => text,
            Err(error) => {
                let _ = std::fs::remove_file(output);
                return Err(format!(
                    "cannot read promotion proof {}: {error}",
                    output.display()
                ));
            }
        };
        let seal: record::Seal = match serde_json::from_str(&text) {
            Ok(seal) => seal,
            Err(error) => {
                let _ = std::fs::remove_file(output);
                return Err(format!(
                    "cannot parse promotion proof {}: {error}",
                    output.display()
                ));
            }
        };
        let audit = tempfile::tempdir()
            .map_err(|error| format!("cannot open a promotion audit seat: {error}"))?;
        if let Err(error) = self.materialize(&seal, &audit.path().join("artifacts")) {
            let _ = std::fs::remove_file(output);
            return Err(error);
        }
        Ok(format!(
            "fetched and audited promotion proof {} {} with {} binary artifacts",
            exact.channel,
            exact.version,
            self.spec.target.len()
        ))
    }

    fn materialize(&self, seal: &record::Seal, artifacts: &Path) -> Result<(), String> {
        if artifacts.exists() {
            return Err(format!(
                "promotion artifact seat already exists: {}",
                artifacts.display()
            ));
        }
        let parent = artifacts.parent().ok_or_else(|| {
            format!(
                "promotion artifact seat has no parent: {}",
                artifacts.display()
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        let stage = tempfile::tempdir_in(parent)
            .map_err(|error| format!("cannot stage promotion artifacts: {error}"))?;
        for target in &self.spec.target {
            let remote = seal.artifacts.get(&target.key).ok_or_else(|| {
                format!(
                    "promotion proof {} misses binary artifact {}",
                    seal.version, target.key
                )
            })?;
            if remote.name != target.archive {
                return Err(format!(
                    "promotion artifact {} is named {}, expected {}",
                    target.key, remote.name, target.archive
                ));
            }
            let path = stage.path().join(&target.archive);
            download(remote, &path)?;
        }
        std::fs::rename(stage.keep(), artifacts)
            .map_err(|error| format!("cannot commit {}: {error}", artifacts.display()))?;
        Ok(())
    }

    fn url(&self, channel: &str, version: &str) -> String {
        format!(
            "{}/v1/releases/{channel}/{version}/seal.json",
            self.spec.authority
        )
    }
}

pub(super) fn derive(
    root: &Path,
    authority: &str,
    commit: &str,
    version: &str,
) -> Result<Exact, String> {
    proof::commit(commit)?;
    channel::intent("stable", version)?;
    let wanted = trunk(version)?;
    let mut found = Vec::new();
    for tag in git::tags(root, commit)? {
        let Ok(channel) = channel::channel(&tag) else {
            continue;
        };
        if channel == "stable" || trunk(&tag)? != wanted {
            continue;
        }
        let url = format!("{authority}/v1/releases/{channel}/{tag}/seal.json");
        if verify::optional(&url)?.is_some() {
            found.push(Exact {
                channel,
                version: tag,
            });
        }
    }
    settle(found, commit, version)
}

fn download(remote: &record::Remote, path: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--retry-all-errors",
            "--retry-delay",
            "1",
            "--output",
        ])
        .arg(path)
        .arg(&remote.url)
        .status()
        .map_err(|error| format!("cannot fetch promotion artifact {}: {error}", remote.name))?;
    if !status.success() {
        return Err(format!(
            "cannot fetch promotion artifact {} from {}",
            remote.name, remote.url
        ));
    }
    let (digest, size) = record::digest(path)?;
    if digest != remote.sha256 || size != remote.size {
        return Err(format!(
            "promotion artifact {} disagrees with its proof",
            remote.name
        ));
    }
    Ok(())
}

fn settle(found: Vec<Exact>, commit: &str, version: &str) -> Result<Exact, String> {
    if found.is_empty() {
        return Err(format!(
            "no published exact seal stands at {commit} for stable {version}"
        ));
    }
    let mut ranked = Vec::new();
    for exact in found {
        let parsed = Version::parse(exact.version.trim_start_matches('v'))
            .map_err(|error| format!("invalid exact version: {error}"))?;
        ranked.push((parsed, exact));
    }
    ranked.sort_by(|left, right| left.0.cmp(&right.0));
    ranked
        .pop()
        .map(|(_, exact)| exact)
        .ok_or_else(|| "promotion source vanished".to_string())
}

fn trunk(version: &str) -> Result<(u64, u64, u64), String> {
    let parsed = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid release version: {error}"))?;
    Ok((parsed.major, parsed.minor, parsed.patch))
}
