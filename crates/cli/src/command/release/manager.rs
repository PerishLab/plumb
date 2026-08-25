use crate::shape::release::Spec;
use std::collections::BTreeMap;
use std::path::Path;

pub fn template() -> Result<String, String> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        crate::command::lane::source::text("assets/manager/unix.sh.in")?.as_bytes(),
    );
    bytes.extend_from_slice(
        crate::command::lane::source::text("assets/manager/windows.ps1.in")?.as_bytes(),
    );
    Ok(super::record::sha(&bytes))
}

pub fn write(spec: &Path, channel: &str, version: &str, out: &Path) -> Result<String, String> {
    let spec = Spec::read(spec)?;
    super::channel::intent(channel, version)?;
    if out.exists() {
        return Err(format!("manager output already exists: {}", out.display()));
    }
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let exact = render(&spec, channel, version)?;
    store(&out.join("manage.sh"), &exact.0, true)?;
    if let Some(windows) = exact.1 {
        store(&out.join("manage.ps1"), &windows, false)?;
    }
    if channel == "stable" {
        let canonical = render(&spec, "stable", "")?;
        let root = out.join("canonical");
        std::fs::create_dir(&root)
            .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
        store(&root.join("manage.sh"), &canonical.0, true)?;
        if let Some(windows) = canonical.1 {
            store(&root.join("manage.ps1"), &windows, false)?;
        }
    }
    Ok(format!(
        "generated {} {} managers in {}",
        channel,
        version,
        out.display()
    ))
}

fn render(spec: &Spec, channel: &str, version: &str) -> Result<(String, Option<String>), String> {
    let mut vars = BTreeMap::from([
        ("product", spec.product.clone()),
        ("environment", spec.environment()),
        ("public_url", spec.authority.clone()),
        ("default_channel", channel.to_string()),
        ("default_version", version.to_string()),
        ("binaries", spec.binaries.join(" ")),
        ("version_probe", "exact-v".to_string()),
        ("unix_platforms", unix(spec)),
        ("windows_archive", String::new()),
        ("windows_key", String::new()),
        ("windows_root", String::new()),
    ]);
    let unix = plumb::fill::fill(
        &crate::command::lane::source::text("assets/manager/unix.sh.in")?,
        &vars,
    )
    .map_err(|error| error.to_string())?;
    let windows = match spec.windows() {
        Some(target) => {
            vars.insert("windows_archive", target.archive.clone());
            vars.insert("windows_key", target.key.clone());
            vars.insert("windows_root", String::new());
            Some(
                plumb::fill::fill(
                    &crate::command::lane::source::text("assets/manager/windows.ps1.in")?,
                    &vars,
                )
                .map_err(|error| error.to_string())?,
            )
        }
        None => None,
    };
    Ok((unix, windows))
}

fn unix(spec: &Spec) -> String {
    let mut lines = Vec::new();
    for target in &spec.target {
        let systems = target
            .systems
            .iter()
            .filter(|held| !held.starts_with("Windows:"))
            .cloned()
            .collect::<Vec<_>>();
        if systems.is_empty() {
            continue;
        }
        lines.push(format!(
            "    {})\n      ARCHIVE={}\n      ARTIFACT={}\n      ARCHIVE_ROOT={}\n      FORMAT={}\n      ;;",
            systems.join("|"),
            target.archive,
            target.key,
            "",
            target.format.name()
        ));
    }
    lines.join("\n")
}

fn store(path: &Path, text: &str, executable: bool) -> Result<(), String> {
    std::fs::write(path, text)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    #[cfg(not(unix))]
    let _ = executable;
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("cannot set mode on {}: {error}", path.display()))?;
    }
    Ok(())
}
