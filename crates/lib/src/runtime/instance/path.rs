use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn current() -> Result<PathBuf, String> {
    Ok(PathBuf::from("/etc/ssh/ssh_host_ed25519_key.pub"))
}

#[cfg(target_os = "windows")]
pub(super) fn current() -> Result<PathBuf, String> {
    let directory = std::env::var_os("ProgramData")
        .filter(|value| !value.is_empty())
        .ok_or("cannot locate OS ProgramData for the SSH host public key")?;
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err("OS ProgramData must be an absolute path".into());
    }
    Ok(directory.join("ssh").join("ssh_host_ed25519_key.pub"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub(super) fn current() -> Result<PathBuf, String> {
    Err("OS instance discovery supports only Linux, macOS and Windows".into())
}
