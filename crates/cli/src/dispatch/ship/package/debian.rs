use crate::dispatch::release::model::Spec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build(
    spec: &Spec,
    version: &str,
    binaries: &BTreeMap<String, PathBuf>,
    output: &Path,
) -> Result<(), String> {
    let deb = spec
        .deb
        .as_ref()
        .ok_or_else(|| "release has no Debian attachment".to_string())?;
    let stage = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = stage.path();
    let payload = deb.root.join("root");
    if payload.exists() {
        copy(&payload, root)?;
    }
    let bin = root.join("usr/bin");
    std::fs::create_dir_all(&bin).map_err(|error| error.to_string())?;
    for (name, source) in binaries {
        let target = bin.join(name);
        std::fs::copy(source, &target)
            .map_err(|error| format!("cannot stage {}: {error}", target.display()))?;
        executable(&target)?;
    }
    let control = root.join("DEBIAN");
    std::fs::create_dir_all(&control).map_err(|error| error.to_string())?;
    let template = std::fs::read_to_string(deb.root.join("control"))
        .map_err(|error| format!("cannot read Debian control template: {error}"))?;
    if template.matches("__VERSION__").count() != 1 {
        return Err("Debian control must contain __VERSION__ exactly once".into());
    }
    std::fs::write(
        control.join("control"),
        template.replace("__VERSION__", &ordered(version)),
    )
    .map_err(|error| format!("cannot write Debian control: {error}"))?;
    for name in ["preinst", "postinst", "prerm", "postrm"] {
        let source = deb.root.join(name);
        if !source.is_file() {
            continue;
        }
        let target = control.join(name);
        std::fs::copy(&source, &target)
            .map_err(|error| format!("cannot stage Debian {name}: {error}"))?;
        executable(&target)?;
    }
    let status = Command::new("dpkg-deb")
        .args(["--root-owner-group", "--build"])
        .arg(root)
        .arg(output)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("TZ", "UTC")
        .status()
        .map_err(|error| format!("cannot run dpkg-deb: {error}"))?;
    if !status.success() {
        return Err("dpkg-deb build failed".into());
    }
    verify(spec, output)
}

pub fn verify(spec: &Spec, path: &Path) -> Result<(), String> {
    let output = Command::new("dpkg-deb")
        .arg("--contents")
        .arg(path)
        .output()
        .map_err(|error| format!("cannot inspect Debian package: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "dpkg-deb inspection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    for binary in &spec.binaries {
        if !listing.contains(&format!("/usr/bin/{binary}")) {
            return Err(format!("Debian package misses usr/bin/{binary}"));
        }
    }
    Ok(())
}

fn ordered(version: &str) -> String {
    let bare = version.trim_start_matches('v');
    match bare.split_once('-') {
        Some((release, prerelease)) => format!("{release}~{prerelease}"),
        None => bare.to_string(),
    }
}

fn copy(source: &Path, target: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        let to = target.join(entry.file_name());
        if kind.is_symlink() {
            return Err(format!(
                "Debian payload refuses symbolic link {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            std::fs::create_dir_all(&to).map_err(|error| error.to_string())?;
            copy(&entry.path(), &to)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &to).map_err(|error| error.to_string())?;
        } else {
            return Err(format!(
                "Debian payload refuses special file {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn executable(path: &Path) -> Result<(), String> {
    #[cfg(not(unix))]
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(
            |error| format!("cannot set executable mode on {}: {error}", path.display()),
        )?;
    }
    Ok(())
}
