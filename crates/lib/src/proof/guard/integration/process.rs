use super::Refusal;
use std::path::Path;
use std::process::{Command, Output};

pub fn run(root: &Path, args: &[&str]) -> Result<Output, Refusal> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| Refusal::new("integration.git", format!("cannot run git: {error}")))
}

pub fn success(
    output: Output,
    code: &'static str,
    message: impl Into<String>,
) -> Result<Vec<u8>, Refusal> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(failed(output, code, message))
}

pub fn failed(output: Output, code: &'static str, message: impl Into<String>) -> Refusal {
    let message = message.into();
    let detail = text(output.stderr);
    Refusal::new(
        code,
        if detail.is_empty() {
            message
        } else {
            format!("{message}: {detail}")
        },
    )
}

pub fn text(bytes: impl AsRef<[u8]>) -> String {
    String::from_utf8_lossy(bytes.as_ref()).trim().to_string()
}
