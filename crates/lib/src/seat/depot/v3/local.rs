use std::path::{Path, PathBuf};

use super::{Bundle, LEAF, POINTER, Pointer};

pub fn generation(root: &Path, digest: &str) -> Result<PathBuf, String> {
    super::value::Value(digest).digest("depot generation")?;
    Ok(root.join("generations").join(digest))
}

pub fn install(root: &Path, pointer: &Pointer, bundle: &Bundle) -> Result<PathBuf, String> {
    let manifest = bundle.manifest.encode()?;
    pointer.bind(&bundle.manifest, &manifest)?;
    if bundle.bodies.len() != bundle.manifest.objects.len() {
        return Err("depot generation bodies do not match its manifest".into());
    }
    let base = generation(root, &pointer.generation)?;
    for object in &bundle.manifest.objects {
        let bytes = bundle
            .bodies
            .get(&object.path)
            .ok_or_else(|| format!("depot generation carries no body for {}", object.path))?;
        bundle
            .manifest
            .verify(&object.path, bytes, object.executable)?;
        write(&base.join(&object.path), bytes, object.executable)?;
    }
    write(&base.join(LEAF), &manifest, false)?;
    write(&root.join(POINTER), &pointer.encode()?, false)?;
    Ok(base)
}

fn write(path: &Path, bytes: &[u8], executable: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    mode(path, executable)
}

#[cfg(unix)]
fn mode(path: &Path, executable: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let value = if executable { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(value))
        .map_err(|error| format!("cannot set mode on {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn mode(_: &Path, _: bool) -> Result<(), String> {
    Ok(())
}
