use std::path::Path;
use std::process::Command;

use plumb::rig::Rig;
use semver::Version;

use super::record::{Capsule, Local, Provenance, Remote, Seal};
use crate::shape::release::Spec;

pub fn run(root: &Path, version: &str, dry: bool) -> Result<String, String> {
    let spec = Spec::read(&root.join("plumb.toml"))?;
    let channel = super::super::channel(version)?;
    let commit = tag(root, version)?;
    let identity = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid adoption version: {error}"))?;
    let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let (cargo, npm) = packages(&spec, &identity, &rig.release.credential)?;
    if cargo.is_empty() && npm.is_empty() {
        return Err("legacy adoption requires at least one external package proof".into());
    }
    let url = format!(
        "{}/v1/releases/{channel}/{version}/seal.json",
        spec.authority
    );
    let seal = Seal {
        schema: 1,
        product: spec.product.clone(),
        channel: channel.clone(),
        version: version.to_string(),
        commit: commit.clone(),
        url: url.clone(),
        generator: crate::command::release::output::generator::resolve(&spec.authority)?,
        provenance: Provenance::Adopted {
            tag: version.to_string(),
            cargo,
            npm,
        },
        artifacts: Default::default(),
        managers: Default::default(),
        legacy: None,
        proof: None,
        radius: None,
        inputs: Default::default(),
    };
    if dry {
        return encode(&seal);
    }
    publish(&spec, seal, &commit, &mut rig)
}

fn publish(spec: &Spec, seal: Seal, commit: &str, rig: &mut Rig) -> Result<String, String> {
    let seat =
        tempfile::tempdir().map_err(|error| format!("cannot stage legacy adoption: {error}"))?;
    let path = seat.path().join("seal.json");
    super::record::json(&path, &seal)?;
    let bytes =
        std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let remote = Remote {
        name: "seal.json".to_string(),
        mime: "application/json; charset=utf-8".to_string(),
        sha256: super::record::sha(&bytes),
        size: bytes.len() as u64,
        url: seal.url.clone(),
    };
    let capsule = Capsule {
        schema: 1,
        product: spec.product.clone(),
        channel: seal.channel.clone(),
        version: seal.version.clone(),
        authority: spec.authority.clone(),
        objects: Vec::new(),
        seal: Local {
            source: "seal.json".to_string(),
            key: format!("v1/releases/{}/{}/seal.json", seal.channel, seal.version),
            remote,
        },
        roots: Vec::new(),
        pointer: None,
    };
    let path = seat.path().join("capsule.json");
    super::record::json(&path, &capsule)?;
    rig.publish.load()?;
    super::storage::publish(&path, &rig.publish)?;
    Ok(format!(
        "adopted legacy release {} at {commit}",
        seal.version
    ))
}

fn packages(
    spec: &Spec,
    version: &Version,
    token: &str,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut cargo = Vec::new();
    if let Some(attachment) = &spec.cargo {
        for package in &attachment.packages {
            crate::command::ship::adaptor::ledger::present(
                crate::command::ship::adaptor::ledger::Presence {
                    spec,
                    cargo: attachment,
                    package,
                    version,
                    token,
                },
            )?;
            cargo.push(package.clone());
        }
    }
    let mut npm = Vec::new();
    if let Some(attachment) = &spec.npm {
        crate::command::ship::adaptor::module::module(spec).present(&version.to_string(), token)?;
        npm.extend(attachment.packages.clone());
    }
    Ok((cargo, npm))
}

fn tag(root: &Path, version: &str) -> Result<String, String> {
    let direct = format!("refs/tags/{version}");
    let peeled = format!("{direct}^{{}}");
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-remote", "--tags", "origin", &direct, &peeled])
        .output()
        .map_err(|error| format!("cannot read remote release tag: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read remote release tag: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    resolve(&text, version)
}

fn resolve(text: &str, version: &str) -> Result<String, String> {
    let mut direct = None;
    let mut peeled = None;
    for line in text.lines() {
        let Some((oid, name)) = line.split_once('\t') else {
            continue;
        };
        if name.ends_with("^{}") {
            peeled = Some(oid.to_string());
        } else {
            direct = Some(oid.to_string());
        }
    }
    peeled
        .or(direct)
        .ok_or_else(|| format!("origin carries no tag {version}"))
}

fn encode(seal: &Seal) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(seal).map_err(|error| error.to_string())?;
    text.push('\n');
    Ok(text)
}
