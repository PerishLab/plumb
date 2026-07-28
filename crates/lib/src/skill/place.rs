use super::agent::Seat;
use super::{Error, Kit};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

const MARK: &str = "metadata.json";

#[derive(Deserialize, Serialize)]
pub struct Mark {
    pub schema: u32,
    pub name: String,
    pub version: String,
    pub keeper: String,
}

pub fn owned(path: &Path, name: &str) -> bool {
    let text = match fs::read_to_string(path.join(MARK)) {
        Ok(text) => text,
        Err(_) => return false,
    };
    match serde_json::from_str::<Mark>(&text) {
        Ok(mark) => mark.keeper == name && mark.name == name,
        Err(_) => false,
    }
}

pub fn place(kit: &Kit, seat: &Seat, bytes: &[u8], version: &str) -> Result<(), Error> {
    let parent = seat
        .path
        .parent()
        .ok_or_else(|| Error::Write(seat.path.clone(), "no parent".to_string()))?;
    let staging = parent.join(format!(".{}-staging", kit.name));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| Error::Write(staging.clone(), error.to_string()))?;
    let built = build(kit, &staging, bytes, version);
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

fn build(kit: &Kit, staging: &Path, bytes: &[u8], version: &str) -> Result<PathBuf, Error> {
    Archive::new(GzDecoder::new(bytes))
        .unpack(staging)
        .map_err(|error| Error::Unpack(error.to_string()))?;
    let root = staging.join(&kit.name);
    if !root.is_dir() {
        return Err(Error::Unpack(format!("archive lacks {}", kit.name)));
    }
    if !root.join("SKILL.md").is_file() {
        return Err(Error::Unpack("archive lacks SKILL.md".to_string()));
    }
    let mark = Mark {
        schema: super::state::SCHEMA,
        name: kit.name.clone(),
        version: version.to_string(),
        keeper: kit.name.clone(),
    };
    let text =
        serde_json::to_string_pretty(&mark).map_err(|error| Error::Parse(error.to_string()))?;
    fs::write(root.join(MARK), format!("{text}\n"))
        .map_err(|error| Error::Write(root.join(MARK), error.to_string()))?;
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
