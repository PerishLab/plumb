use plumb::config::Environment;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn command() -> Command {
    let cargo = &crate::catalog::set::current().stable.cargo;
    configured(&cargo.registry, &cargo.index)
}

pub fn configure(command: &mut Command) {
    let cargo = &crate::catalog::set::current().stable.cargo;
    for (key, value) in configured(&cargo.registry, &cargo.index).get_envs() {
        if let Some(value) = value {
            command.env(key, value);
        }
    }
}

fn configured(registry: &str, index: &str) -> Command {
    let mut command = plumb::config::detached("cargo");
    command.env(
        format!(
            "CARGO_REGISTRIES_{}_INDEX",
            registry.to_ascii_uppercase().replace('-', "_")
        ),
        index,
    );
    command
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
