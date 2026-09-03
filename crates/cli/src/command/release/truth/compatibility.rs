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

fn version(raw: &str, label: &str) -> Result<semver::Version, String> {
    semver::Version::parse(raw.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse {label} {raw}: {error}"))
}
