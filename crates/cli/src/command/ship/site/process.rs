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

pub fn held(call: Call<'_>) -> Result<String, String> {
    let bin = call.bin;
    let output = Command::new(bin)
        .args(call.args)
        .current_dir(call.cwd)
        .envs(call.env.iter().copied())
        .output()
        .map_err(|error| format!("cannot run {bin}: {error}"))?;
    let shown = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !shown.is_empty() {
        println!("{shown}");
    }
    if output.status.success() {
        return Ok(shown);
    }
    Err(said(bin, &output.stderr, &shown, output.status.code()))
}

pub fn text(bin: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("cannot run {bin}: {error}"))?;
    let shown = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        return Ok(shown);
    }
    Err(said(bin, &output.stderr, &shown, output.status.code()))
}

fn said(bin: &str, stderr: &[u8], shown: &str, code: Option<i32>) -> String {
    let held = String::from_utf8_lossy(stderr).trim().to_string();
    let held = if held.is_empty() {
        shown.to_string()
    } else {
        held
    };
    let held = if held.is_empty() {
        "it said nothing".to_string()
    } else {
        held
    };
    match code {
        Some(code) => format!("{bin} failed with {code}: {held}"),
        None => format!("{bin} failed: {held}"),
    }
}
