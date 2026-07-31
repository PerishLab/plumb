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

pub fn commit(value: &str) -> Result<(), String> {
    if (40..=64).contains(&value.len())
        && value
            .bytes()
            .all(|held| held.is_ascii_hexdigit() && !held.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err("pick requires one full lowercase --commit SHA".into())
    }
}
