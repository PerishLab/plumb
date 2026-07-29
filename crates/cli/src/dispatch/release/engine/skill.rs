use super::super::model::Spec;
use super::archive::{Member, bundle};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

pub fn build(spec: &Spec, version: &str, output: &Path) -> Result<(), String> {
    let source = spec.root.join("skills").join(&spec.product);
    let mut members = BTreeMap::new();
    collect(&source, &source, &spec.product, &mut members)?;
    let metadata = serde_json::json!({
        "schema": 1,
        "name": spec.product,
        "version": version,
        "keeper": spec.product,
    });
    members.insert(
        PathBuf::from(&spec.product).join("metadata.json"),
        Member {
            bytes: serde_json::to_vec_pretty(&metadata).map_err(|error| error.to_string())?,
            mode: 0o644,
        },
    );
    bundle(output, &members)?;
    verify(spec, output)
}

pub fn verify(spec: &Spec, path: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut names = Vec::new();
    for entry in archive
        .entries()
        .map_err(|error| format!("cannot read skill archive: {error}"))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        names.push(
            entry
                .path()
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .to_string(),
        );
    }
    for member in [
        format!("{}/SKILL.md", spec.product),
        format!("{}/metadata.json", spec.product),
    ] {
        if !names.iter().any(|name| name == &member) {
            return Err(format!("skill archive misses {member}"));
        }
    }
    Ok(())
}

fn collect(
    root: &Path,
    source: &Path,
    product: &str,
    members: &mut BTreeMap<PathBuf, Member>,
) -> Result<(), String> {
    let mut entries = std::fs::read_dir(source)
        .map_err(|error| format!("cannot read skill source {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "skill source refuses symbolic link {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            collect(root, &entry.path(), product, members)?;
        } else if kind.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_path_buf();
            let mode = mode(&entry)?;
            let bytes = std::fs::read(entry.path()).map_err(|error| error.to_string())?;
            members.insert(
                PathBuf::from(product).join(relative),
                Member { bytes, mode },
            );
        } else {
            return Err(format!(
                "skill source refuses special file {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn mode(entry: &std::fs::DirEntry) -> Result<u32, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let held = entry
            .metadata()
            .map_err(|error| error.to_string())?
            .permissions()
            .mode();
        Ok(if held & 0o111 == 0 { 0o644 } else { 0o755 })
    }
    #[cfg(not(unix))]
    {
        entry.metadata().map_err(|error| error.to_string())?;
        Ok(0o644)
    }
}
