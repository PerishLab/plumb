use crate::catalog::rules::release as law;
use plumb::land::rejoin::tags;
use semver::Version;

pub fn version(raw: &str, channel: &str) -> Result<String, String> {
    let value = if raw.starts_with('v') {
        raw.to_string()
    } else if raw.is_empty() {
        String::new()
    } else {
        format!("v{raw}")
    };
    let seen = value
        .strip_prefix('v')
        .ok_or_else(|| "version is required".to_string())?;
    if seen.is_empty() {
        return Err("version is required".into());
    }
    let parsed = Version::parse(seen).map_err(|_| format!("invalid exact version: {value}"))?;
    if channel == "stable" {
        if !parsed.pre.is_empty() {
            return Err(format!("version {value} does not belong to channel stable"));
        }
    } else {
        exact(channel)?;
        let prefix = format!("{channel}.");
        let turn = parsed.pre.as_str().strip_prefix(&prefix).unwrap_or("");
        if turn.is_empty()
            || turn.starts_with('0')
            || !turn.bytes().all(|held| held.is_ascii_digit())
        {
            return Err(format!(
                "version {value} does not belong to channel {channel}"
            ));
        }
    }
    Ok(value)
}

pub fn ascends(listing: &str, remote: &str, marker: &str) -> Result<(), String> {
    let read = |name: &str| Version::parse(name.strip_prefix('v')?).ok();
    let claimed = read(marker).ok_or_else(|| format!("invalid exact version: {marker}"))?;
    let highest = tags(listing)
        .into_iter()
        .filter(|(name, _)| *name != marker)
        .filter_map(|(name, _)| Some((read(name)?, name)))
        .max_by(|left, right| left.0.cmp(&right.0));
    match highest {
        Some((held, name)) if held >= claimed => Err(format!(
            "{marker} does not exceed {name}, the highest marker {remote} holds; a release version only ascends [{}]",
            law::MARKER_ORDERED.0
        )),
        _ => Ok(()),
    }
}

pub fn exact(channel: &str) -> Result<(), String> {
    let mut chars = channel.chars();
    if !chars.next().is_some_and(|held| held.is_ascii_lowercase())
        || !chars.all(|held| held.is_ascii_lowercase() || held.is_ascii_digit() || held == '-')
    {
        return Err(format!("invalid release channel: {channel}"));
    }
    Ok(())
}

pub fn branch(version: &str) -> String {
    format!("release/{version}")
}
