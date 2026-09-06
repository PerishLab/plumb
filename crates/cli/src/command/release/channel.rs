use semver::Version;

pub(in crate::command) fn base(value: &str) -> Result<String, String> {
    let raw = value
        .strip_prefix('v')
        .ok_or_else(|| format!("release version must begin with v: {value}"))?;
    let version =
        Version::parse(raw).map_err(|error| format!("invalid release version: {error}"))?;
    Ok(format!(
        "v{}.{}.{}",
        version.major, version.minor, version.patch
    ))
}

pub(in crate::command) fn channel(value: &str) -> Result<String, String> {
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

pub(in crate::command) fn intent(channel: &str, value: &str) -> Result<(), String> {
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
