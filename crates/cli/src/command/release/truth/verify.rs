use super::record::{Capsule, Local, Pointer, Remote, Seal};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(path: &Path, activated: bool) -> Result<String, String> {
    let (capsule, root) = Capsule::read(path)?;
    published(&capsule)?;
    if activated {
        projection(&capsule)?;
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

pub fn projection(capsule: &Capsule) -> Result<(), String> {
    if capsule.channel != "stable" {
        return Err("only stable has an activated binary surface".into());
    }
    for manager in &capsule.roots {
        local(manager)?;
    }
    Ok(())
}

pub fn inspect(url: &str, stable: bool) -> Result<String, String> {
    Surface(url).release(stable)
}

pub fn optional(url: &str) -> Result<Option<serde_json::Value>, String> {
    let path = temporary("probe");
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--write-out",
            "%{http_code}",
            "--output",
        ])
        .arg(&path)
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(format!(
            "cannot probe {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let held = match status.as_str() {
        "200" => parse(&path).map(Some),
        "404" => Ok(None),
        code => Err(format!("cannot probe {url}: HTTP {code}")),
    };
    let _ = std::fs::remove_file(path);
    held
}

pub fn binary(url: &str, stable: bool) -> Result<String, String> {
    Surface(url).binary(stable)
}

pub(in crate::command::release) struct Surface<'a>(pub(in crate::command::release) &'a str);

impl Surface<'_> {
    pub(in crate::command::release) fn sealed(
        &self,
        binary: bool,
    ) -> Result<(Seal, String), String> {
        let path = self.download("depot-release")?;
        let held = (|| {
            let seal: Seal = parse(&path)?;
            if seal.url != self.0 {
                return Err("exact seal URL disagrees with its record".into());
            }
            if binary {
                audit(&seal)?;
            } else {
                current(&seal)?;
            }
            let (digest, _) = super::record::digest(&path)?;
            Ok((seal, digest))
        })();
        let _ = std::fs::remove_file(path);
        held
    }

    pub(in crate::command::release) fn digest(&self) -> Result<String, String> {
        let path = self.download("generator")?;
        let held = super::record::digest(&path).map(|(digest, _)| digest);
        let _ = std::fs::remove_file(path);
        held
    }

    fn release(&self, stable: bool) -> Result<String, String> {
        if !stable {
            let seal: Seal = self.read()?;
            if seal.url != self.0 {
                return Err("exact seal URL disagrees with its record".into());
            }
            current(&seal)?;
            return Ok(format!(
                "inspected release {} {}",
                seal.channel, seal.version
            ));
        }
        let pointer: Pointer = self.read()?;
        if pointer.schema != 1 || pointer.channel != "stable" {
            return Err("stable pointer is not current".into());
        }
        let seal: Seal = remote(&pointer.seal)?;
        let valid = seal.schema == 1 && seal.channel == "stable";
        let product = seal.product == pointer.product && seal.version == pointer.version;
        if !valid || !product || seal.commit != pointer.commit {
            return Err("stable pointer and exact seal disagree".into());
        }
        current(&seal)?;
        Ok(format!(
            "inspected stable release {} {}",
            pointer.product, pointer.version
        ))
    }

    fn binary(&self, stable: bool) -> Result<String, String> {
        if !stable {
            let seal: Seal = self.read()?;
            audit(&seal)?;
            return Ok(format!(
                "inspected binary {} {}",
                seal.channel, seal.version
            ));
        }
        let pointer: Pointer = self.read()?;
        let seal: Seal = remote(&pointer.seal)?;
        audit(&seal)?;
        for manager in pointer.managers.values() {
            prove(manager)?;
        }
        Ok(format!(
            "inspected stable binary {} {}",
            pointer.product, pointer.version
        ))
    }

    fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        let path = self.download("record")?;
        let result = parse(&path);
        let _ = std::fs::remove_file(path);
        result
    }

    fn download(&self, name: &str) -> Result<PathBuf, String> {
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
            .arg(self.0)
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        if !output.status.success() {
            let _ = std::fs::remove_file(&path);
            return Err(format!(
                "cannot read back {}: {}",
                self.0,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(path)
    }
}

fn current(seal: &Seal) -> Result<(), String> {
    if seal.schema != 1 || seal.channel.is_empty() || seal.version.is_empty() {
        return Err("exact seal is not current".into());
    }
    super::super::output::generator::audit(seal)
}

fn audit(seal: &Seal) -> Result<(), String> {
    current(seal)?;
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

fn parse<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

pub fn object(held: &Local) -> Result<(), String> {
    local(held)
}

pub fn describe(source: &str, name: &str, mime: &str) -> Result<Remote, String> {
    let path = Surface(source).download(name)?;
    let held = super::record::digest(&path).map(|(sha256, size)| Remote {
        name: name.to_string(),
        mime: mime.to_string(),
        sha256,
        size,
        url: source.to_string(),
    });
    let _ = std::fs::remove_file(path);
    held
}

fn local(object: &Local) -> Result<(), String> {
    let path = fetch(&object.remote)?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

pub(super) fn fetch(remote: &Remote) -> Result<PathBuf, String> {
    let path = Surface(&remote.url).download(&remote.name)?;
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

fn temporary(name: &str) -> PathBuf {
    PathBuf::from(format!("plumb-release-{}-{}", std::process::id(), name))
}
