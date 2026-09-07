use sha2::{Digest as _, Sha256};

pub(super) fn digest(
    name: &str,
    input: &str,
    commands: &[Vec<String>],
    binding: &super::action::Binding<'_>,
) -> Result<String, String> {
    let mut sponge = Sha256::new();
    sponge.update(name.as_bytes());
    sponge.update([0]);
    sponge.update(input.as_bytes());
    sponge.update([0]);
    sponge.update(serde_json::to_vec(commands).map_err(|error| error.to_string())?);
    sponge.update([0]);
    sponge.update(plumb::version!("PLUMB").as_bytes());
    if let Some(commit) = plumb::commit!("PLUMB") {
        sponge.update([0]);
        sponge.update(commit.as_bytes());
    }
    sponge.update([0]);
    let rules = match binding.configuration {
        Some(configuration) => configuration.to_string(),
        None => plumb::depot::rules()?.mark().to_string(),
    };
    sponge.update(rules.as_bytes());
    if name == "guard/plumb"
        && let Some(configuration) = binding.configuration
    {
        sponge.update([0]);
        sponge.update(configuration.as_bytes());
    }
    if let Some(profile) = binding.profile {
        sponge.update([0]);
        sponge.update(profile.digest.as_bytes());
    }
    sponge.update([0]);
    sponge.update(plumb::config::platform().as_bytes());
    if let Some(execution) = binding.execution {
        sponge.update([0]);
        sponge.update(execution.evidence()?);
    }
    for tool in tools(name) {
        sponge.update([0]);
        sponge.update(tool.as_bytes());
        sponge.update([0]);
        sponge.update(version(tool, binding.execution)?.as_bytes());
    }
    Ok(format!("{:x}", sponge.finalize()))
}

fn tools(name: &str) -> &'static [&'static str] {
    match name {
        "guard/rust" | "guard/test" => &["cargo", "rustc"],
        "guard/web" => &["node", "pnpm"],
        "guard/ectropy" => &["ectropy"],
        "guard/plumb" => &["plumb"],
        _ => &[],
    }
}

pub(super) fn execution(
    name: &str,
    environment: plumb::config::Environment,
    root: &std::path::Path,
) -> Result<plumb::config::Execution, String> {
    let mut programs = tools(name)
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    programs.extend(["cargo".to_string(), "rustc".to_string()]);
    programs.sort();
    programs.dedup();
    plumb::config::Execution::new(environment, &programs, root)
}

fn version(tool: &str, execution: Option<&plumb::config::Execution>) -> Result<String, String> {
    let mut command = match execution {
        Some(execution) => execution.command(tool)?,
        None => plumb::config::detached(tool),
    };
    let output = command
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run {tool} --version: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{tool} --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
