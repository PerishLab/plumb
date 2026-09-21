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
    sponge.update(plumb::depot::rules()?.mark().as_bytes());
    sponge.update([0]);
    sponge.update(plumb::config::platform().as_bytes());
    if let Some(execution) = binding.execution {
        sponge.update([0]);
        sponge.update(execution.evidence()?);
        sponge.update([0]);
        sponge.update(crate::catalog::probe::evidence(binding.probes, execution)?);
    }
    for tool in tools(name) {
        if binding.execution.is_some()
            && crate::catalog::probe::covers(binding.probes, &[tool, "--version"])?
        {
            continue;
        }
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
    selected: &[String],
) -> Result<plumb::config::Execution, String> {
    let mut programs = tools(name)
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    programs.extend(selected.iter().cloned());
    programs.sort();
    programs.dedup();
    plumb::config::Execution::new(environment, &programs, root)
}

pub(super) fn environment(
    commands: &[Vec<String>],
    captured: Option<plumb::config::Environment>,
) -> Result<plumb::config::Environment, String> {
    if let Some(captured) = captured {
        return Ok(captured);
    }
    let name = if commands
        .iter()
        .filter_map(|command| command.first())
        .any(|program| crate::execution::family(program) == "pnpm")
    {
        "pnpm"
    } else {
        "probe"
    };
    crate::execution::environment(name)
}

fn version(tool: &str, execution: Option<&plumb::config::Execution>) -> Result<String, String> {
    let output = match execution {
        Some(execution) => execution.output(&[tool.into(), "--version".into()]),
        None => plumb::config::detached(tool)
            .arg("--version")
            .output()
            .map_err(|error| error.to_string()),
    }
    .map_err(|error| format!("cannot run {tool} --version: {error}"))?;
    if output.status.success() {
        let stdout = std::str::from_utf8(&output.stdout)
            .map_err(|_| format!("{tool} --version stdout is not UTF-8"))?;
        Ok(stdout
            .replace("\r\n", "\n")
            .trim_end_matches('\n')
            .to_string())
    } else {
        Err(format!(
            "{tool} --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
