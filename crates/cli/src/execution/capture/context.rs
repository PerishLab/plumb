use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Admission {
    pub tool: plumb::config::Tool,
    pub directory: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub tool: plumb::config::Tool,
    pub directory: PathBuf,
    pub arguments: Vec<String>,
    environment: BTreeMap<String, String>,
}

impl Context {
    pub(super) fn capture(admission: &Admission, arguments: &[OsString]) -> Result<Self, String> {
        let directory = std::env::current_dir().map_err(|error| error.to_string())?;
        if directory != admission.directory
            || plumb::config::Tool::at(admission.tool.path.clone())? != admission.tool
        {
            return Err("Cargo runner differs from the admitted program or directory".into());
        }
        let mut environment = BTreeMap::new();
        for (key, value) in std::env::vars_os() {
            if environment.insert(unicode(key)?, unicode(value)?).is_some() {
                return Err("Cargo runner environment repeats an input".into());
            }
        }
        let context = Self {
            tool: admission.tool.clone(),
            directory,
            arguments: arguments
                .iter()
                .cloned()
                .map(unicode)
                .collect::<Result<_, _>>()?,
            environment,
        };
        context.validate()?;
        Ok(context)
    }

    pub(super) fn agrees(&self, admission: &Admission) -> bool {
        self.tool == admission.tool && self.directory == admission.directory
    }

    pub fn command(&self, threads: usize) -> Result<Command, String> {
        self.validate()?;
        if plumb::config::Tool::at(self.tool.path.clone())? != self.tool {
            return Err("captured test program changed before execution".into());
        }
        let environment = self.environment()?;
        let program = self
            .tool
            .path
            .to_str()
            .ok_or("captured executable is not Unicode")?
            .to_string();
        let execution = plumb::config::Execution::new(
            environment,
            std::slice::from_ref(&program),
            &self.directory,
        )?;
        let mut command = execution.command(&program)?;
        command
            .envs(&self.environment)
            .args(&self.arguments)
            .args(["--test-threads", &threads.to_string()]);
        Ok(command)
    }

    fn validate(&self) -> Result<(), String> {
        if self
            .environment
            .iter()
            .any(|(key, value)| key.starts_with("LISTEN_") || value.contains("--jobserver-"))
        {
            return Err("Cargo runner context carries descriptor-dependent state".into());
        }
        if self
            .arguments
            .iter()
            .any(|argument| argument.starts_with("--test-threads"))
        {
            return Err("Cargo runner context already sets a test thread budget".into());
        }
        self.environment().map(|_| ())
    }

    fn environment(&self) -> Result<plumb::config::Environment, String> {
        let contract = plumb::config::Contract {
            inherit: self.environment.keys().cloned().collect(),
            managed: Vec::new(),
            reject: Vec::new(),
            bind: BTreeMap::new(),
        };
        contract.capture(
            self.environment
                .iter()
                .map(|(key, value)| (OsString::from(key), OsString::from(value))),
        )
    }
}

fn unicode(value: OsString) -> Result<String, String> {
    value
        .into_string()
        .map_err(|_| "Cargo runner context is not Unicode".into())
}

pub(super) fn directory(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("cannot resolve runner directory: {error}"))
}
