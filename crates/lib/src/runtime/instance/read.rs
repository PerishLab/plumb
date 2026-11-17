use std::fs::File;
use std::io::Read;
use std::path::Path;

const LIMIT: u64 = 16 * 1024;

pub(super) fn public(path: &Path) -> Result<Vec<u8>, String> {
    if path.extension() != Some(std::ffi::OsStr::new("pub")) {
        return Err("instance identification reads only .pub public-key files".into());
    }
    let before = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "cannot inspect SSH host public key {}: {error}",
            path.display()
        )
    })?;
    if !before.is_file() || before.len() > LIMIT {
        return Err("SSH host public key must be a regular file of at most 16 KiB".into());
    }
    let file = File::open(path).map_err(|error| {
        format!(
            "cannot read SSH host public key {}: {error}",
            path.display()
        )
    })?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect SSH host public key: {error}"))?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("SSH host public key must be a regular file of at most 16 KiB".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot finish reading SSH host public key: {error}"))?;
    if bytes.len() as u64 > LIMIT {
        return Err("SSH host public key grew beyond 16 KiB while reading".into());
    }
    Ok(bytes)
}
