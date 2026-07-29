use super::super::model::Format;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    let file =
        File::create(path).map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    for (name, source) in binaries {
        archive
            .append_path_with_name(source, name)
            .map_err(|error| format!("cannot archive {name}: {error}"))?;
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
