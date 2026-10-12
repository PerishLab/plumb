use super::agent::Seat;
use super::state::SCHEMA;
use super::{Error, Kit};
use crate::record;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

const MARK: &str = "metadata.json";

#[derive(Deserialize, Serialize)]
pub struct Mark {
    pub name: String,
    pub version: String,
    pub keeper: String,
}

pub fn owned(path: &Path, name: &str) -> bool {
    marker(path, name).is_some()
}

pub fn marker(path: &Path, name: &str) -> Option<Mark> {
    record::read::<Mark>(&path.join(MARK), SCHEMA)
        .ok()
        .flatten()
        .filter(|mark| mark.keeper == name && mark.name == name)
}

pub fn place(kit: &Kit, seat: &Seat, bytes: &[u8], version: &str) -> Result<(), Error> {
    let parent = seat
        .path
        .parent()
        .ok_or_else(|| Error::Write(seat.path.clone(), "no parent".to_string()))?;
    let staging = parent.join(format!(".{}-staging-{}", kit.name, std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| Error::Write(staging.clone(), error.to_string()))?;
    let mark = Mark {
        name: kit.name.clone(),
        version: version.to_string(),
        keeper: kit.name.clone(),
    };
    let built = build(&staging, bytes, mark);
    let root = match built {
        Ok(root) => root,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    let swapped = swap(&root, &seat.path);
    let _ = fs::remove_dir_all(&staging);
    swapped
}

pub fn stage(kit: &Kit, seat: &Seat, bytes: &[u8], version: &str) -> Result<(), Error> {
    if seat.path.exists() {
        return Err(Error::Occupied(seat.path.clone()));
    }
    let parent = seat
        .path
        .parent()
        .ok_or_else(|| Error::Write(seat.path.clone(), "no parent".to_string()))?;
    let staging = parent.join(format!(".{}-candidate-{}", kit.name, std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| Error::Write(staging.clone(), error.to_string()))?;
    let mark = Mark {
        name: kit.name.clone(),
        version: version.to_string(),
        keeper: format!("{}-stage", kit.name),
    };
    let built = build(&staging, bytes, mark);
    let root = match built {
        Ok(root) => root,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if seat.path.exists() {
        let _ = fs::remove_dir_all(&staging);
        return Err(Error::Occupied(seat.path.clone()));
    }
    if let Some(parent) = seat.path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::Write(parent.to_path_buf(), error.to_string()))?;
    }
    let landed = fs::rename(&root, &seat.path)
        .map_err(|error| Error::Write(seat.path.clone(), error.to_string()));
    let _ = fs::remove_dir_all(&staging);
    landed
}

fn build(staging: &Path, bytes: &[u8], mark: Mark) -> Result<PathBuf, Error> {
    Archive::new(GzDecoder::new(bytes))
        .unpack(staging)
        .map_err(|error| Error::Unpack(error.to_string()))?;
    let root = staging.join(&mark.name);
    if !root.is_dir() {
        return Err(Error::Unpack(format!("archive lacks {}", mark.name)));
    }
    if !root.join("SKILL.md").is_file() {
        return Err(Error::Unpack("archive lacks SKILL.md".to_string()));
    }
    record::write(&root.join(MARK), SCHEMA, &mark).map_err(super::state::fault)?;
    Ok(root)
}

fn swap(root: &Path, seat: &Path) -> Result<(), Error> {
    if let Some(parent) = seat.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::Write(parent.to_path_buf(), error.to_string()))?;
    }
    if seat.exists() {
        fs::remove_dir_all(seat)
            .map_err(|error| Error::Write(seat.to_path_buf(), error.to_string()))?;
    }
    fs::rename(root, seat).map_err(|error| Error::Write(seat.to_path_buf(), error.to_string()))
}

pub fn erase(path: &Path) -> Result<(), Error> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::Write(path.to_path_buf(), error.to_string())),
    }
}
