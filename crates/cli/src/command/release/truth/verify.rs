use super::record::{Pointer, Remote, Seal};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(in crate::command::release) struct Surface<'a>(pub(in crate::command::release) &'a str);

impl Surface<'_> {
    pub(in crate::command::release) fn pointer(&self, channel: &str) -> Result<Pointer, String> {
        let pointer: Pointer = self.read()?;
        if pointer.schema != 1 || pointer.channel != channel {
            return Err(format!("{channel} pointer is not current"));
        }
        Ok(pointer)
    }

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
                "6",
                "--retry-all-errors",
                "--retry-delay",
                "2",
                "--retry-max-time",
                "30",
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

fn parse<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
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
