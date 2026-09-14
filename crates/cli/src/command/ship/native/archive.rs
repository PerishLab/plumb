use crate::shape::release::Format;
use std::{fs::File, io::Read, path::Path};

pub(super) fn read(format: Format, path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    match format {
        Format::Tar => tar(file, name),
        Format::Zip => zip(file, name),
    }
}

fn tar(file: File, name: &str) -> Result<Vec<u8>, String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut found = None;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        if entry.path().map_err(|error| error.to_string())? != Path::new(name) {
            continue;
        }
        if found.is_some() || !entry.header().entry_type().is_file() {
            return Err("binary archive member is duplicated or not a regular file".into());
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        found = Some(bytes);
    }
    found.ok_or_else(|| format!("binary archive carries no {name}"))
}

fn zip(file: File, name: &str) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    if archive.file_names().filter(|held| *held == name).count() != 1 {
        return Err("binary archive member is absent or duplicated".into());
    }
    let mut entry = archive.by_name(name).map_err(|error| error.to_string())?;
    if !entry.is_file()
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err("binary archive member is not a regular file".into());
    }
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}
