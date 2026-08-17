use super::model::Spec;
use semver::Version;
use std::collections::BTreeMap;
use std::path::Path;

const UNIX: &str = include_str!("../../../assets/manager/unix.sh.in");
const WINDOWS: &str = include_str!("../../../assets/manager/windows.ps1.in");

pub fn template() -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(UNIX.as_bytes());
    bytes.extend_from_slice(WINDOWS.as_bytes());
    super::record::sha(&bytes)
}

pub fn write(spec: &Path, channel: &str, version: &str, out: &Path) -> Result<String, String> {
    let spec = Spec::read(spec)?;
    intent(channel, version)?;
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
    let unix = plumb::fill::fill(UNIX, &vars).map_err(|error| error.to_string())?;
    let windows = match spec.windows() {
        Some(target) => {
            vars.insert("windows_archive", target.archive.clone());
            vars.insert("windows_key", target.key.clone());
            vars.insert("windows_root", String::new());
            Some(plumb::fill::fill(WINDOWS, &vars).map_err(|error| error.to_string())?)
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

pub(super) fn channel(value: &str) -> Result<String, String> {
    let raw = value
        .strip_prefix('v')
        .ok_or_else(|| format!("release version must begin with v: {value}"))?;
    let version =
        Version::parse(raw).map_err(|error| format!("invalid release version: {error}"))?;
    if version.pre.is_empty() {
        return Ok("stable".into());
    }
    let parts = version.pre.as_str().split('.').collect::<Vec<_>>();
    let named = parts.first().copied().unwrap_or_default();
    let number = parts
        .get(1)
        .and_then(|held| held.parse::<u64>().ok())
        .unwrap_or_default();
    if parts.len() != 2 || named.is_empty() || number == 0 {
        return Err(format!("version {value} names no channel"));
    }
    intent(named, value)?;
    Ok(named.into())
}

pub(super) fn intent(channel: &str, value: &str) -> Result<(), String> {
    if channel.is_empty()
        || !channel
            .chars()
            .next()
            .is_some_and(|held| held.is_ascii_lowercase())
        || !channel
            .chars()
            .all(|held| held.is_ascii_lowercase() || held.is_ascii_digit() || held == '-')
    {
        return Err(format!("invalid release channel: {channel}"));
    }
    let raw = value
        .strip_prefix('v')
        .ok_or_else(|| format!("release version must begin with v: {value}"))?;
    let version =
        Version::parse(raw).map_err(|error| format!("invalid release version: {error}"))?;
    if channel == "stable" {
        if !version.pre.is_empty() {
            return Err(format!(
                "stable release cannot use prerelease version {value}"
            ));
        }
    } else {
        let parts = version.pre.as_str().split('.').collect::<Vec<_>>();
        let number = parts
            .get(1)
            .and_then(|held| held.parse::<u64>().ok())
            .unwrap_or_default();
        if parts.len() != 2 || parts[0] != channel || number == 0 {
            return Err(format!(
                "version {value} does not belong to channel {channel}"
            ));
        }
    }
    Ok(())
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
