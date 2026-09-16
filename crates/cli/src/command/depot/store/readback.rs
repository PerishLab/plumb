use std::process::Command;

pub(super) fn immutable(
    bundle: &plumb::depot::v3::Bundle,
    source: &str,
    base: &str,
) -> Result<(), String> {
    for (path, bytes) in &bundle.bodies {
        public(&format!("{source}/{base}/objects/{path}"), bytes)?;
    }
    public(
        &format!("{source}/{base}/{}", plumb::depot::v3::LEAF),
        &bundle.manifest.encode()?,
    )
}

pub(super) fn public(url: &str, expected: &[u8]) -> Result<(), String> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "3",
            "--max-time",
            "10",
            "--retry",
            "2",
            "--retry-delay",
            "1",
            "--retry-all-errors",
        ])
        .arg(url)
        .output()
        .map_err(|error| format!("cannot read back {url}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read back {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if output.stdout != expected {
        return Err(format!("public depot object drift: {url}"));
    }
    Ok(())
}
