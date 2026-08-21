use crate::command::release::model::Format;
use flate2::{Compression, GzBuilder};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(super) struct Member {
    pub bytes: Vec<u8>,
    pub mode: u32,
}

pub fn write(
    format: Format,
    path: &Path,
    binaries: &BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    match format {
        Format::Tar => tar(path, binaries),
        Format::Zip => zip(path, binaries),
    }
}

pub fn names(format: Format, path: &Path) -> Result<Vec<String>, String> {
    match format {
        Format::Tar => {
            let file = File::open(path)
                .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            let mut names = Vec::new();
            for entry in archive.entries().map_err(|error| error.to_string())? {
                names.push(
                    entry
                        .map_err(|error| error.to_string())?
                        .path()
                        .map_err(|error| error.to_string())?
                        .to_string_lossy()
                        .to_string(),
                );
            }
            Ok(names)
        }
        Format::Zip => {
            let file = File::open(path)
                .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
            let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
            (0..archive.len())
                .map(|index| {
                    archive
                        .by_index(index)
                        .map(|entry| entry.name().to_string())
                        .map_err(|error| error.to_string())
                })
                .collect()
        }
    }
}

fn tar(path: &Path, binaries: &BTreeMap<String, PathBuf>) -> Result<(), String> {
    let members = binaries
        .iter()
        .map(|(name, source)| {
            let bytes = std::fs::read(source)
                .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
            Ok((PathBuf::from(name), Member { bytes, mode: 0o755 }))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    bundle(path, &members)
}

pub(super) fn bundle(path: &Path, members: &BTreeMap<PathBuf, Member>) -> Result<(), String> {
    let file =
        File::create(path).map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    let encoder = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    for (name, member) in members {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(member.mode);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_size(member.bytes.len() as u64);
        header.set_cksum();
        archive
            .append_data(&mut header, name, member.bytes.as_slice())
            .map_err(|error| format!("cannot archive {}: {error}", name.display()))?;
    }
    let encoder = archive
        .into_inner()
        .map_err(|error| format!("cannot finish {}: {error}", path.display()))?;
    encoder
        .finish()
        .map_err(|error| format!("cannot finish {}: {error}", path.display()))?;
    Ok(())
}

fn zip(path: &Path, binaries: &BTreeMap<String, PathBuf>) -> Result<(), String> {
    let file =
        File::create(path).map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o755);
    for (name, source) in binaries {
        archive
            .start_file(format!("{name}.exe"), options)
            .map_err(|error| error.to_string())?;
        let bytes = std::fs::read(source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        archive
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
    }
    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}
