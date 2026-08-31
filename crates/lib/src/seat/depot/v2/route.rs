use super::value::Value;
use super::{Kind, Release};

pub fn snapshots(release: &Release, derivative: Kind, timestamp: &str) -> Result<String, String> {
    release.validate()?;
    Value(timestamp).component("snapshot timestamp")?;
    Ok(format!(
        "v2/products/{}/derivatives/{}/releases/{}/{}/snapshots/{timestamp}",
        release.product,
        derivative.label(),
        release.channel,
        release.version
    ))
}

pub fn latest(product: &str, derivative: Kind, channel: &str) -> Result<String, String> {
    Value(product).component("release product")?;
    Value(channel).component("release channel")?;
    Ok(format!(
        "v2/products/{product}/derivatives/{}/channels/{channel}/latest.json",
        derivative.label()
    ))
}

pub fn exact(
    product: &str,
    derivative: Kind,
    channel: &str,
    version: &str,
) -> Result<String, String> {
    Value(product).component("release product")?;
    Value(channel).component("release channel")?;
    Value(version).component("release version")?;
    semver::Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse release version {version}: {error}"))?;
    Ok(format!(
        "v2/products/{product}/derivatives/{}/releases/{channel}/{version}/latest.json",
        derivative.label()
    ))
}
