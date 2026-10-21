use super::{Manifest, SCHEMA, Snapshot};
use std::collections::BTreeSet;
use std::io::Read as _;
use std::path::{Component, Path};

pub(super) fn read(path: &Path, identity: &str) -> Result<Option<Snapshot>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("compilation parcel is not a regular file".into());
    }
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let decoder = zstd::Decoder::new(file).map_err(|error| error.to_string())?;
    let mut tar = tar::Archive::new(decoder.take(32 * 1024 * 1024 * 1024));
    tar.set_preserve_permissions(true);
    tar.set_preserve_ownerships(false);
    tar.set_overwrite(false);
    let root = tempfile::tempdir_in(path.parent().ok_or("compilation parcel has no parent")?)
        .map_err(|error| error.to_string())?;
    let mut names = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut manifest = None;
    let mut remaining = 32 * 1024 * 1024 * 1024_u64;
    for entry in tar.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .path()
            .map_err(|error| error.to_string())?
            .into_owned();
        validate(&name)?;
        if !names.insert(name.clone()) {
            return Err("compilation parcel repeats an entry".into());
        }
        let kind = entry.header().entry_type();
        if kind != tar::EntryType::Regular && !kind.is_dir() {
            return Err("compilation parcel contains a link or special file".into());
        }
        if name == Path::new("manifest.json") {
            if kind != tar::EntryType::Regular || entry.size() > 8 * 1024 * 1024 {
                return Err("invalid compilation parcel manifest".into());
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            manifest = Some(
                serde_json::from_slice::<Manifest>(&bytes).map_err(|error| error.to_string())?,
            );
            continue;
        }
        remaining = remaining
            .checked_sub(entry.size())
            .ok_or("compilation parcel exceeds 32 GiB of decoded material")?;
        if kind == tar::EntryType::Regular {
            files.insert(name.clone());
        }
        if !entry
            .unpack_in(root.path())
            .map_err(|error| error.to_string())?
        {
            return Err("compilation parcel entry escaped its staging root".into());
        }
    }
    let manifest = manifest.ok_or("compilation parcel has no manifest")?;
    if manifest.schema != SCHEMA || manifest.identity != identity {
        return Err("compilation parcel identity does not match this Guard context".into());
    }
    let timed = manifest
        .times
        .keys()
        .map(Path::new)
        .map(Path::to_path_buf)
        .collect::<BTreeSet<_>>();
    if timed != files {
        return Err("compilation parcel timestamps do not match its files".into());
    }
    for (name, stamp) in &manifest.times {
        let file = std::fs::File::options()
            .read(true)
            .write(cfg!(windows))
            .open(root.path().join(name))
            .map_err(|error| error.to_string())?;
        file.set_modified(stamp.time()?)
            .map_err(|error| error.to_string())?;
    }
    Ok(Some(Snapshot { root, manifest }))
}

fn validate(name: &Path) -> Result<(), String> {
    if !name
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("compilation parcel path is not relative".into());
    }
    if name == Path::new("manifest.json") {
        return Ok(());
    }
    if !name.starts_with("cargo") && !name.starts_with("compiler") {
        return Err("compilation parcel contains undeclared content".into());
    }
    Ok(())
}
