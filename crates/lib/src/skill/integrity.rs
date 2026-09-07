use super::{Error, Record, fetch::Grant};
use crate::depot::v3::{Generation, Query};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn held(grant: &Grant, record: &Record) -> Result<bool, Error> {
    let mut actual = BTreeSet::new();
    if std::fs::symlink_metadata(&record.path)
        .map_err(|error| Error::Read(record.path.clone(), error.to_string()))?
        .file_type()
        .is_symlink()
        || !leaves(&record.path, &record.path, &mut actual)?
    {
        return Ok(false);
    }
    let target = grant
        .generation
        .as_ref()
        .expect("a Depot generation update");
    let (source, _) = grant
        .url
        .rsplit_once("/channels/")
        .ok_or_else(|| Error::Shape(grant.url.clone()))?;
    let route = crate::depot::v3::Route::new(
        &target.manifest.channel,
        target.manifest.kind,
        &record.version,
    );
    let url = crate::depot::v3::manifest(source, route, &record.sha).map_err(Error::Shape)?;
    if url.strip_suffix(crate::depot::v3::LEAF) != Some(record.url.as_str()) {
        return Ok(false);
    }
    let prior = Generation::named(
        Query {
            source,
            product: &target.manifest.product,
            channel: &target.manifest.channel,
            version: &record.version,
            kind: target.manifest.kind,
        },
        &record.sha,
    )
    .map_err(|error| Error::Fetch(record.url.clone(), error))?;
    if prior.manifest.marker != target.manifest.marker {
        return Ok(false);
    }
    let mut expected = BTreeSet::from(["metadata.json".to_string()]);
    for object in &prior.manifest.objects {
        let path = record.path.join(&object.path);
        let bytes =
            std::fs::read(&path).map_err(|error| Error::Read(path.clone(), error.to_string()))?;
        if crate::depot::sha(&bytes) != object.sha256 {
            return Ok(false);
        }
        expected.insert(object.path.clone());
    }
    Ok(actual == expected)
}

fn leaves(root: &Path, path: &Path, found: &mut BTreeSet<String>) -> Result<bool, Error> {
    let entries = std::fs::read_dir(path)
        .map_err(|error| Error::Read(path.to_path_buf(), error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::Read(path.to_path_buf(), error.to_string()))?;
        let child = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| Error::Read(child.clone(), error.to_string()))?;
        if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
            return Ok(false);
        }
        if kind.is_dir() {
            if !leaves(root, &child, found)? {
                return Ok(false);
            }
        } else {
            found.insert(
                child
                    .strip_prefix(root)
                    .expect("skill child")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(true)
}
