use std::{path::Path, process::Command};

pub(super) fn finalize(path: &Path, target: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
    }
    if target.contains("apple-darwin") {
        if !cfg!(target_os = "macos") {
            return Err("Mach-O identity finalization requires its native runner".into());
        }
        for args in [
            vec!["--force", "--sign", "-", "--timestamp=none"],
            vec!["--verify", "--strict"],
        ] {
            let status = Command::new("codesign")
                .args(args)
                .arg(path)
                .status()
                .map_err(|error| format!("cannot finalize Mach-O signature: {error}"))?;
            if !status.success() {
                return Err("Mach-O signature finalization failed".into());
            }
        }
    }
    Ok(())
}

pub(super) fn probe(path: &Path, product: &str, marker: &str) -> Result<(), String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run bound executable: {error}"))?;
    let expected = format!("{product} {marker}");
    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != expected {
        return Err(format!("bound executable does not report {expected}"));
    }
    Ok(())
}
