use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
pub struct Search {
    pub(super) root: PathBuf,
    pub(super) path: Option<OsString>,
    pub(super) extensions: Option<OsString>,
}

impl Search {
    pub fn new(
        root: &Path,
        path: Option<OsString>,
        extensions: Option<OsString>,
    ) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("cannot resolve search root: {error}"))?;
        if !root.is_dir() {
            return Err(format!(
                "search root is not a directory: {}",
                root.display()
            ));
        }
        Ok(Self {
            root,
            path,
            extensions,
        })
    }

    pub fn resolve(&self, program: impl AsRef<OsStr>) -> Result<PathBuf, String> {
        let program = program.as_ref();
        let path = which::WhichConfig::new_with_sys(self)
            .custom_cwd(self.root.clone())
            .binary_name(program.to_os_string())
            .first_result()
            .map_err(|error| {
                format!("cannot resolve tool {}: {error}", program.to_string_lossy())
            })?;
        if cfg!(windows)
            && let Some(extension) = path.extension()
            && !["exe", "com", "cmd", "bat"]
                .iter()
                .any(|name| extension.eq_ignore_ascii_case(name))
        {
            return Err(format!("unsupported executable format: {}", path.display()));
        }
        Ok(path)
    }

    pub fn command(&self, program: impl AsRef<OsStr>) -> Result<Command, String> {
        let mut command = Command::new(self.resolve(program)?);
        command.current_dir(&self.root);
        match &self.path {
            Some(path) => {
                command.env("PATH", path);
            }
            None => {
                command.env_remove("PATH");
            }
        }
        if cfg!(windows) {
            match &self.extensions {
                Some(extensions) => {
                    command.env("PATHEXT", extensions);
                }
                None => {
                    command.env_remove("PATHEXT");
                }
            }
        }
        Ok(command)
    }
}
