use super::marker::Seat;
use plumb::forgejo::git;
use std::path::Path;
use std::process::{Command, Output};

impl Seat {
    pub(super) fn fetch(&self) -> Result<(), String> {
        git::fetch(&self.root)?;
        success(
            "fetch release markers",
            Command::new("git")
                .args(["fetch", "--tags", "origin"])
                .current_dir(&self.root)
                .output(),
        )
        .map(|_| ())
    }

    pub(super) fn read<const N: usize>(&self, args: [&str; N]) -> Result<String, String> {
        let output = command(&self.root, args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

pub(super) fn command<const N: usize>(root: &Path, args: [&str; N]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: std::io::Result<Output>) -> Result<String, String> {
    let output = output.map_err(|error| format!("cannot run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
