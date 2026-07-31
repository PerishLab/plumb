use std::path::Path;
use std::process::Command;

pub struct Call<'a> {
    pub bin: &'a str,
    pub args: &'a [&'a str],
    pub cwd: &'a Path,
    pub env: &'a [(&'a str, &'a str)],
}

pub fn run(call: Call<'_>) -> Result<(), String> {
    let status = Command::new(call.bin)
        .args(call.args)
        .current_dir(call.cwd)
        .envs(call.env.iter().copied())
        .status()
        .map_err(|error| format!("cannot run {}: {error}", call.bin))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with {status}", call.bin))
    }
}

pub fn text(bin: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("cannot run {bin}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{bin} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
