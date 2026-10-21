use super::inputs::{digest, mode};
use super::stamp::Stamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

pub(crate) const NAMES: [&str; 4] = ["debug", "release", ".rustc_info.json", "CACHEDIR.TAG"];

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Seal {
    pub sha: String,
    pub files: u64,
    pub latest: Stamp,
    pub links: u64,
}

pub(crate) struct Target<'a>(pub &'a Path);

struct Hash<'a> {
    root: &'a Path,
    hash: Sha256,
    files: u64,
    latest: Stamp,
    links: u64,
}

impl Target<'_> {
    pub fn supported(&self) -> Result<bool, String> {
        if !self.0.exists() {
            return Ok(true);
        }
        for entry in std::fs::read_dir(self.0).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let metadata = entry.file_type().map_err(|error| error.to_string())?;
            let name = entry.file_name();
            if metadata.is_dir()
                && !["debug", "release", "tmp"]
                    .iter()
                    .any(|known| name == *known)
            {
                return Ok(false);
            }
            if metadata.is_symlink() && NAMES.iter().any(|known| name == *known) {
                return Err(
                    "Guard compilation transport refuses a foreign material symlink".into(),
                );
            }
        }
        Ok(true)
    }

    pub fn seal(&self) -> Result<Seal, String> {
        let mut hash = Hash {
            root: self.0,
            hash: Sha256::new(),
            files: 0,
            links: 0,
            latest: Stamp::at(std::time::UNIX_EPOCH)?,
        };
        for name in NAMES {
            let path = self.0.join(name);
            if std::fs::symlink_metadata(&path).is_ok() {
                hash.absorb(&path)?;
            }
        }
        Ok(Seal {
            sha: format!("{:x}", hash.hash.finalize()),
            files: hash.files,
            latest: hash.latest,
            links: hash.links,
        })
    }

    pub fn reset(&self) -> Result<(), String> {
        for name in ["debug", "release", ".rustc_info.json"] {
            let path = self.0.join(name);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.to_string()),
            };
            if metadata.file_type().is_symlink() {
                return Err("Guard compilation reset refuses a foreign material symlink".into());
            }
            let result = if metadata.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            result.map_err(|error| {
                format!(
                    "cannot invalidate compilation material {}: {error}",
                    path.display()
                )
            })?;
        }
        Ok(())
    }
}

impl Hash<'_> {
    fn absorb(&mut self, path: &Path) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        let name = path
            .strip_prefix(self.root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("compilation material path is not UTF-8")?;
        self.hash.update(name.as_bytes());
        self.hash.update([0]);
        self.hash.update(mode(&metadata).to_le_bytes());
        if metadata.is_dir() {
            self.hash.update(b"directory\0");
            let mut entries = std::fs::read_dir(path)
                .map_err(|error| error.to_string())?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<PathBuf>, _>>()
                .map_err(|error| error.to_string())?;
            entries.sort();
            for entry in entries {
                self.absorb(&entry)?;
            }
        } else {
            self.file(path, &metadata)?;
        }
        Ok(())
    }

    fn file(&mut self, path: &Path, metadata: &std::fs::Metadata) -> Result<(), String> {
        self.files += 1;
        self.latest = self.latest.max(Stamp::at(
            metadata.modified().map_err(|error| error.to_string())?,
        )?);
        if metadata.file_type().is_symlink() {
            self.links += 1;
            self.hash.update(b"symlink\0");
            let link = std::fs::read_link(path).map_err(|error| error.to_string())?;
            self.hash.update(
                link.to_str()
                    .ok_or("compilation material link is not UTF-8")?
                    .as_bytes(),
            );
        } else if metadata.is_file() {
            self.hash.update(b"file\0");
            self.hash.update(metadata.len().to_le_bytes());
            self.hash.update(digest(path)?.as_bytes());
        } else {
            return Err("compilation material contains an unsupported file".into());
        }
        self.hash.update([0]);
        Ok(())
    }
}
