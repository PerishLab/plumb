use std::fs;
use std::io::Write;
use std::path::Path;

pub fn write(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    land(target, bytes, None)
}

pub fn private(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    land(target, bytes, Some(0o600))
}

fn land(target: &Path, bytes: &[u8], mode: Option<u32>) -> std::io::Result<()> {
    if let Some(parent) = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let draft = target.with_file_name(format!(".{name}.{}.part", std::process::id()));
    let landed = fill(&draft, bytes, mode).and_then(|()| fs::rename(&draft, target));
    if landed.is_err() {
        let _ = fs::remove_file(&draft);
    }
    landed
}

fn fill(draft: &Path, bytes: &[u8], mode: Option<u32>) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let mut file = open(&mut options, draft, mode)?;
    file.write_all(bytes)
}

#[cfg(unix)]
fn open(
    options: &mut fs::OpenOptions,
    draft: &Path,
    mode: Option<u32>,
) -> std::io::Result<fs::File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let Some(mode) = mode else {
        return options.open(draft);
    };
    let file = options.mode(mode).open(draft)?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    Ok(file)
}

#[cfg(not(unix))]
fn open(options: &mut fs::OpenOptions, draft: &Path, _: Option<u32>) -> std::io::Result<fs::File> {
    options.open(draft)
}
