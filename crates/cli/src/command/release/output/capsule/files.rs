use crate::command::release::artifact::Asset;
use crate::command::release::model::Spec;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn declared(spec: &Spec, assets: &[Asset]) -> BTreeSet<String> {
    spec.target
        .iter()
        .map(|held| held.archive.clone())
        .chain(assets.iter().map(|held| held.file.clone()))
        .collect()
}

pub fn complete(root: &Path, declared: &BTreeSet<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("cannot read artifact root {}: {error}", root.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read artifact entry: {error}"))?;
        if !entry.path().is_file() {
            return Err(format!(
                "artifact root contains non-file {}",
                entry.path().display()
            ));
        }
        found.insert(entry.file_name().to_string_lossy().to_string());
    }
    if &found != declared {
        return Err(format!(
            "artifact set disagrees: expected {declared:?}, found {found:?}"
        ));
    }
    Ok(())
}

pub fn stage(from: &Path, to: &Path, name: &str) -> Result<PathBuf, String> {
    let source = from.join(name);
    let target = to.join(name);
    std::fs::copy(&source, &target).map_err(|error| {
        format!(
            "cannot stage artifact {} as {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(target)
}
