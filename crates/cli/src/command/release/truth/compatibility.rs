#[derive(Clone, Copy)]
pub(super) enum Channel {
    Beta,
    Stable,
}

pub(super) fn channel(target: &str, beta: &str, stable: &str) -> Result<Channel, String> {
    let target = version(target, "guard target")?;
    let beta = version(beta, "released validator")?;
    let stable = version(stable, "released validator")?;
    match (
        beta.major == target.major && beta <= target,
        stable.major == target.major && stable <= target,
    ) {
        (true, true) if beta > stable => Ok(Channel::Beta),
        (true, true) | (false, true) => Ok(Channel::Stable),
        (true, false) => Ok(Channel::Beta),
        (false, false) => Err(format!(
            "released validator v{stable} cannot open v{target}"
        )),
    }
}

pub(super) fn related(target: &str, validator: &str) -> Result<(), String> {
    let target = version(target, "guard target")?;
    let validator = version(validator, "released validator")?;
    if (validator.major, validator.minor, validator.patch)
        != (target.major, target.minor, target.patch)
        || validator.pre.is_empty()
    {
        return Err(format!(
            "released validator v{validator} does not belong to v{target}"
        ));
    }
    Ok(())
}

pub(super) fn recovery(
    binding: &Binding,
    executable: &Path,
    binary: &str,
) -> Result<String, String> {
    if !executable.is_absolute() || !executable.is_file() {
        return Err("guard recovery validator must be an absolute executable path".into());
    }
    let output = plumb::config::detached(executable)
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot identify guard recovery validator: {error}"))?;
    let identity = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let expected = format!("{binary} {}", binding.release.version);
    if !output.status.success() || identity != expected {
        return Err(format!(
            "guard recovery validator identifies as {identity:?}, expected {expected:?}"
        ));
    }
    let (artifact, _) = super::record::digest(executable)?;
    eprintln!(
        "guard recovery validator {} ({artifact})",
        executable.display()
    );
    Ok(artifact)
}

fn version(raw: &str, label: &str) -> Result<semver::Version, String> {
    semver::Version::parse(raw.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse {label} {raw}: {error}"))
}
use std::path::Path;

use super::depot::Binding;
