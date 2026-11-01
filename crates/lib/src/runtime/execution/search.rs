use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
pub struct Search(radicula_process::Search);

impl Search {
    pub fn new(
        root: &Path,
        path: Option<OsString>,
        extensions: Option<OsString>,
    ) -> Result<Self, String> {
        radicula_process::Search::new(root, path, extensions)
            .map(Self)
            .map_err(|error| format!("cannot resolve search root: {error}"))
    }

    pub fn resolve(&self, program: impl AsRef<OsStr>) -> Result<PathBuf, String> {
        let program = program.as_ref();
        self.0
            .resolve(program)
            .map_err(|error| format!("cannot resolve tool {}: {error}", program.to_string_lossy()))
    }

    pub fn command(&self, program: impl AsRef<OsStr>) -> Result<Command, String> {
        let program = program.as_ref();
        self.0
            .command(program)
            .map_err(|error| format!("cannot resolve tool {}: {error}", program.to_string_lossy()))
    }
}
