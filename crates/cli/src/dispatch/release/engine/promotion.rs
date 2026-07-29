use super::super::{manager, model::Spec, verify};
use std::path::Path;
use std::process::Command;

pub fn fetch(spec: &Spec, channel: &str, version: &str, output: &Path) -> Result<String, String> {
    manager::intent(channel, version)?;
    if channel == "stable" {
        return Err("stable promotion source must be non-stable".into());
    }
    if output.exists() {
        return Err(format!(
            "promotion proof already exists: {}",
            output.display()
        ));
    }
    let url = format!(
        "{}/v1/releases/{channel}/{version}/seal.json",
        spec.authority
    );
    verify::inspect(&url, false)?;
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(output)
        .arg(&url)
        .status()
        .map_err(|error| format!("cannot fetch promotion proof: {error}"))?;
    if !status.success() {
        return Err(format!("cannot fetch promotion proof from {url}"));
    }
    Ok(format!("fetched promotion proof {channel} {version}"))
}
