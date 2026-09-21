pub fn sourced(running: &str) -> bool {
    running.trim_start_matches('v') == "0.0.0"
}

pub fn related(running: &str, released: &str) -> Result<bool, String> {
    let running = semver::Version::parse(running.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse running Plumb version: {error}"))?;
    let released = semver::Version::parse(released.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse depot release version: {error}"))?;
    if running == released {
        return Ok(true);
    }
    let core = (running.major, running.minor, running.patch);
    let candidate = (released.major, released.minor, released.patch);
    Ok(match (running.pre.is_empty(), released.pre.is_empty()) {
        (true, false) => core == candidate,
        _ => false,
    })
}
