use std::process::Command;

use crate::shape::depot::Batch;

pub(super) fn prove(
    plan: &Batch,
    base: &str,
    manifest: &str,
    pointer: Option<&str>,
) -> Result<(), String> {
    let source = plan.manifest.source.trim_end_matches('/');
    for (path, bytes) in &plan.bodies {
        public(&format!("{source}/{base}/{path}"), bytes)?;
    }
    public(
        &format!("{source}/{base}/{}", plumb::depot::v2::LEAF),
        manifest.as_bytes(),
    )?;
    if let Some(pointer) = pointer {
        let key = plumb::depot::v2::latest(
            &plan.manifest.release.product,
            plan.manifest.derivative,
            &plan.manifest.release.channel,
        )?;
        public(&format!("{source}/{key}"), pointer.as_bytes())?;
    }
    Ok(())
}

fn public(url: &str, expected: &[u8]) -> Result<(), String> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "6",
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
