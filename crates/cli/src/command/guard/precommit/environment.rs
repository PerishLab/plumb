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
    Configuration(&home).absent()?;
    let parent = physical(root)?;
    for parent in parent.ancestors() {
        Configuration(&parent.join(".cargo")).absent()?;
    }
    Ok(())
}

fn physical(root: &Path) -> Result<PathBuf, String> {
    for candidate in root.ancestors() {
        match candidate.canonicalize() {
            Ok(path) if candidate == root => {
                return path
                    .parent()
                    .map(Path::to_path_buf)
                    .ok_or_else(|| "Cargo execution root has no parent".to_string());
            }
            Ok(path) => return Ok(path),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && matches!(std::fs::symlink_metadata(candidate), Err(error)
                        if error.kind() == std::io::ErrorKind::NotFound) => {}
            Err(error) => {
                return Err(format!(
                    "cannot resolve Cargo execution path {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot resolve Cargo execution path {}",
        root.display()
    ))
}

struct Configuration<'a>(&'a Path);

impl Configuration<'_> {
    fn absent(&self) -> Result<(), String> {
        for name in ["config", "config.toml"] {
            let path = self.0.join(name);
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
}
