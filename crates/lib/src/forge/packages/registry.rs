use super::process;
use semver::Version;
use std::path::Path;

pub(super) fn latest(root: &Path, ecosystem: &str, name: &str) -> Result<String, String> {
    if ecosystem == "npm" {
        let argv = ["pnpm", "view", name, "dist-tags.latest", "--json"].map(str::to_string);
        let bytes = process::run(root, "pnpm", &argv)?;
        let version: String = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot read npm stable {name}: {error}"))?;
        stable(&version)?;
        return Ok(version);
    }
    let url = format!("https://cargo.perish.uk/{}", route(name));
    let argv = [
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--retry",
        "1",
        "--connect-timeout",
        "5",
        "--max-time",
        "15",
        &url,
    ]
    .map(str::to_string);
    let bytes = process::run(root, "probe", &argv)?;
    cargo(&bytes)
}

fn stable(text: &str) -> Result<Version, String> {
    let version =
        Version::parse(text).map_err(|error| format!("invalid stable version {text}: {error}"))?;
    if !version.pre.is_empty() {
        return Err(format!("registry latest {text} is a prerelease"));
    }
    Ok(version)
}

pub(super) fn cargo(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let mut latest = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let item: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid registry line: {error}"))?;
        let text = item
            .get("vers")
            .and_then(serde_json::Value::as_str)
            .ok_or("registry line has no version")?;
        let version = Version::parse(text).map_err(|error| error.to_string())?;
        if item.get("yanked").and_then(serde_json::Value::as_bool) == Some(true)
            || !version.pre.is_empty()
        {
            continue;
        }
        if latest.as_ref().is_none_or(|held| &version > held) {
            latest = Some(version);
        }
    }
    latest
        .map(|version| version.to_string())
        .ok_or_else(|| "registry exposes no stable version".into())
}

fn route(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}
