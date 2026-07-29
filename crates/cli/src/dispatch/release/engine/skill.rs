use super::super::model::Spec;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::path::Path;

pub fn build(spec: &Spec, version: &str, output: &Path) -> Result<(), String> {
    let source = spec.root.join("skills").join(&spec.product);
    let stage = tempfile::tempdir().map_err(|error| error.to_string())?;
    let target = stage.path().join(&spec.product);
    copy(&source, &target)?;
    let metadata = serde_json::json!({
        "schema": 1,
        "name": spec.product,
        "version": version,
        "keeper": spec.product,
    });
    std::fs::write(
        target.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot write skill metadata: {error}"))?;
    let file = File::create(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive
        .append_dir_all(&spec.product, &target)
        .map_err(|error| format!("cannot archive skill: {error}"))?;
    let encoder = archive
        .into_inner()
        .map_err(|error| format!("cannot finish skill archive: {error}"))?;
    encoder
        .finish()
        .map_err(|error| format!("cannot finish skill compression: {error}"))?;
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

fn copy(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source)
        .map_err(|error| format!("cannot read skill source {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        let to = target.join(entry.file_name());
        if kind.is_symlink() {
            return Err(format!(
                "skill source refuses symbolic link {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            copy(&entry.path(), &to)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), to).map_err(|error| error.to_string())?;
        } else {
            return Err(format!(
                "skill source refuses special file {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}
