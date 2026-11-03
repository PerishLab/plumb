use super::context::{Admission, Context, directory};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Declaration {
    schema: String,
    runner: plumb::config::Tool,
    pub programs: Vec<Admission>,
}

pub(crate) struct Session {
    root: tempfile::TempDir,
    declaration: Declaration,
}

impl Session {
    pub fn new(
        programs: impl Iterator<Item = (PathBuf, PathBuf)>,
        carrier: &[String],
    ) -> Result<Self, String> {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        protect(root.path())?;
        let programs = programs
            .map(|(path, cwd)| {
                Ok(Admission {
                    tool: plumb::config::Tool::at(directory(&path)?)?,
                    directory: directory(&cwd)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let declaration = Declaration {
            schema: "plumb.test-context/v1".into(),
            runner: plumb::config::Tool::at(directory(Path::new(
                carrier
                    .first()
                    .ok_or("Cargo capture requires an owned runner")?,
            ))?)?,
            programs,
        };
        declaration.validate()?;
        write(&root.path().join("session.json"), &declaration)?;
        Ok(Self { root, declaration })
    }

    pub fn configuration(&self, carrier: &[String]) -> Result<String, String> {
        if carrier.is_empty() {
            return Err("Cargo capture requires an owned runner".into());
        }
        if plumb::config::Tool::at(directory(Path::new(&carrier[0]))?)? != self.declaration.runner {
            return Err("Cargo capture runner changed before launch".into());
        }
        let mut arguments = carrier.to_vec();
        arguments.push(
            directory(self.root.path())?
                .to_str()
                .ok_or("runner directory is not Unicode")?
                .to_string(),
        );
        let arguments =
            toml::Value::Array(arguments.into_iter().map(toml::Value::String).collect());
        Ok(format!("target.'cfg(all())'.runner={arguments}"))
    }

    pub fn contexts(&self) -> Result<Vec<Context>, String> {
        let root = directory(self.root.path())?;
        let entries = std::fs::read_dir(&root)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        if entries.len() != self.declaration.programs.len() + 1 {
            return Err("Cargo capture does not cover exactly the admitted programs".into());
        }
        self.declaration
            .programs
            .iter()
            .enumerate()
            .map(|(index, admission)| {
                let context: Context = read(&root.join(format!("{index}.json")))?;
                if !context.agrees(admission) {
                    return Err("captured Cargo context differs from its admission".into());
                }
                Ok(context)
            })
            .collect()
    }
}

impl Declaration {
    pub fn read(root: &Path) -> Result<Self, String> {
        private(root)?;
        let declaration: Self = read(&root.join("session.json"))?;
        declaration.validate()?;
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        if plumb::config::Tool::at(directory(&executable)?)? != declaration.runner {
            return Err("Cargo capture callback has a different runner image".into());
        }
        Ok(declaration)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != "plumb.test-context/v1" {
            return Err("unsupported Cargo runner declaration".into());
        }
        let unique = self
            .programs
            .iter()
            .map(|program| &program.tool.path)
            .collect::<std::collections::BTreeSet<_>>();
        if unique.len() != self.programs.len() {
            return Err("Cargo runner declaration repeats an executable".into());
        }
        Ok(())
    }
}

pub(super) fn write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    restrict(&mut options);
    let mut file = options
        .open(path)
        .map_err(|error| format!("cannot create Cargo context record: {error}"))?;
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Cargo context record is not a regular file".into());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("invalid Cargo context record: {error}"))
}

#[cfg(unix)]
fn private(root: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(root).map_err(|error| error.to_string())?;
    let owner = tempfile::tempfile()
        .and_then(|file| file.metadata())
        .map_err(|error| error.to_string())?
        .uid();
    if !metadata.is_dir() || metadata.mode() & 0o077 != 0 || metadata.uid() != owner {
        return Err("Cargo context directory is not private to the runner identity".into());
    }
    Ok(())
}

#[cfg(not(unix))]
fn private(_: &Path) -> Result<(), String> {
    Err("Cargo context capture is not qualified on this platform".into())
}

#[cfg(unix)]
pub(super) fn protect(root: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
pub(super) fn protect(_: &Path) -> Result<(), String> {
    Err("Cargo context capture is not qualified on this platform".into())
}

#[cfg(unix)]
fn restrict(options: &mut std::fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn restrict(_: &mut std::fs::OpenOptions) {}
