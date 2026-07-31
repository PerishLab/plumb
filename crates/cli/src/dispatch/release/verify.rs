use super::record::{Capsule, Local, Pointer, Remote, Seal};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(path: &Path, activated: bool) -> Result<String, String> {
    let (capsule, root) = Capsule::read(path)?;
    published(&capsule)?;
    if activated {
        activation(&capsule)?;
    }
    Ok(format!(
        "verified {} {} from {}",
        capsule.channel,
        capsule.version,
        root.display()
    ))
}

pub fn published(capsule: &Capsule) -> Result<(), String> {
    local(&capsule.seal)?;
    for object in &capsule.objects {
        local(object)?;
    }
    Ok(())
}

pub fn activation(capsule: &Capsule) -> Result<(), String> {
    if capsule.channel != "stable" {
        return Err("only stable has an activation surface".into());
    }
    let pointer = capsule
        .pointer
        .as_ref()
        .ok_or_else(|| "stable capsule has no pointer".to_string())?;
    local(pointer)?;
    for manager in &capsule.roots {
        local(manager)?;
    }
    Ok(())
}

pub fn inspect(url: &str, stable: bool) -> Result<String, String> {
    if stable {
        let pointer: Pointer = read(url)?;
        if pointer.schema != 1 || pointer.channel != "stable" {
            return Err("stable pointer is not current".into());
        }
        let seal: Seal = remote(&pointer.seal)?;
        let current = seal.schema == 1 && seal.channel == "stable";
        let product = seal.product == pointer.product && seal.version == pointer.version;
        if !current || !product || seal.commit != pointer.commit {
            return Err("stable pointer and exact seal disagree".into());
        }
        audit(&seal)?;
        for manager in pointer.managers.values() {
            prove(manager)?;
        }
        Ok(format!(
            "inspected stable {} {}",
            pointer.product, pointer.version
        ))
    } else {
        let seal: Seal = read(url)?;
        if seal.url != url {
            return Err("exact seal URL disagrees with its record".into());
        }
        audit(&seal)?;
        Ok(format!("inspected {} {}", seal.channel, seal.version))
    }
}

fn audit(seal: &Seal) -> Result<(), String> {
    if seal.schema != 1 || seal.channel.is_empty() || seal.version.is_empty() {
        return Err("exact seal is not current".into());
    }
    for object in seal.artifacts.values().chain(seal.managers.values()) {
        prove(object)?;
    }
    Ok(())
}

fn prove(remote: &Remote) -> Result<(), String> {
    let path = fetch(remote)?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

fn remote<T: serde::de::DeserializeOwned>(remote: &Remote) -> Result<T, String> {
    let path = fetch(remote)?;
    let result = parse(&path);
    let _ = std::fs::remove_file(path);
    result
}

fn read<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let path = download(url, "record")?;
    let result = parse(&path);
    let _ = std::fs::remove_file(path);
    result
}

fn parse<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

fn local(object: &Local) -> Result<(), String> {
    let path = fetch(&object.remote)?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

fn fetch(remote: &Remote) -> Result<PathBuf, String> {
    let path = download(&remote.url, &remote.name)?;
    let result = super::record::digest(&path).and_then(|(digest, size)| {
        if digest == remote.sha256 && size == remote.size {
            Ok(path.clone())
        } else {
            Err(format!("public object drift: {}", remote.url))
        }
    });
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}

fn download(url: &str, name: &str) -> Result<PathBuf, String> {
    let path = temporary(name);
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--output",
        ])
        .arg(&path)
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(format!(
            "cannot read back {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(path)
}

fn temporary(name: &str) -> PathBuf {
    PathBuf::from(format!("plumb-release-{}-{}", std::process::id(), name))
}
