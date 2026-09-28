use std::path::{Path, PathBuf};

use plumb::config::{Environment, Execution, Tool};

pub(crate) fn programs() -> &'static [&'static str] {
    if cfg!(windows) {
        &["cl", "lib", "link", "mt", "nmake", "rc"]
    } else {
        &[]
    }
}

pub(crate) fn environment(environment: &mut Environment) -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let mut roots = Vec::new();
    for key in ["VCToolsInstallDir", "WindowsSdkDir"] {
        let Some(value) = environment.get(key).filter(|value| !value.is_empty()) else {
            return Err(format!(
                "Windows MSVC Cargo execution requires {key}; initialize the Visual Studio C++ build environment"
            ));
        };
        if !Path::new(value).is_absolute() {
            return Err(format!(
                "Windows MSVC Cargo execution requires an absolute {key}"
            ));
        }
        roots.push(PathBuf::from(value));
    }
    environment.prioritize(&roots)
}

pub(crate) fn tools(execution: &Execution) -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let tools = execution.tools()?;
    let vc = canonical(execution.environment.get("VCToolsInstallDir").unwrap())?;
    let sdk = canonical(execution.environment.get("WindowsSdkDir").unwrap())?;
    for name in ["cl", "lib", "link", "nmake"] {
        below(&tools, name, &vc, "VCToolsInstallDir")?;
    }
    for name in ["mt", "rc"] {
        below(&tools, name, &sdk, "WindowsSdkDir")?;
    }
    Ok(())
}

fn canonical(path: &str) -> Result<PathBuf, String> {
    Path::new(path)
        .canonicalize()
        .map_err(|error| format!("cannot resolve native toolchain root {path}: {error}"))
}

fn below(
    tools: &std::collections::BTreeMap<String, Tool>,
    name: &str,
    root: &Path,
    authority: &str,
) -> Result<(), String> {
    let tool = tools
        .get(name)
        .ok_or_else(|| format!("Windows MSVC Cargo execution did not bind {name}"))?;
    let path = tool.path.canonicalize().map_err(|error| {
        format!(
            "cannot resolve native tool {}: {error}",
            tool.path.display()
        )
    })?;
    if !path.starts_with(root) {
        return Err(format!(
            "Windows MSVC Cargo execution requires {name} below {authority}; resolved {}",
            path.display()
        ));
    }
    Ok(())
}
