use plumb::config::{Contract, Environment};
use std::path::{Path, PathBuf};

pub(super) fn cargo(root: &Path) -> Result<Environment, String> {
    let rules = crate::catalog::set::read("workflow")?;
    let contract: Contract = rules
        .get("execution")
        .and_then(|held| held.get("cargo"))
        .ok_or_else(|| "rules/workflow.toml must declare execution.cargo".to_string())?
        .clone()
        .try_into()
        .map_err(|error| format!("invalid Cargo execution contract: {error}"))?;
    let environment = plumb::config::environment(&contract)?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve Cargo root: {error}"))?;
    inspect(&root, &environment)?;
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "Cargo execution has no managed home".to_string())?;
    inspect(&home.join("tmp/guard"), &environment)?;
    Ok(environment)
}

pub(super) fn inspect(root: &Path, environment: &Environment) -> Result<(), String> {
    let home = environment
        .get("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            environment
                .get(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| Path::new(home).join(".cargo"))
        })
        .ok_or_else(|| "Cargo execution cannot determine its home".to_string())?;
    if !home.is_absolute() {
        return Err("Cargo execution requires an absolute home".into());
    }
    absent(&home)?;
    for parent in root.ancestors().skip(1) {
        absent(&parent.join(".cargo"))?;
    }
    Ok(())
}

fn absent(root: &Path) -> Result<(), String> {
    for name in ["config", "config.toml"] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                return Err(format!(
                    "Cargo execution refuses unbound host configuration {}; use the Plumb-owned configuration",
                    path.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
        }
    }
    Ok(())
}
