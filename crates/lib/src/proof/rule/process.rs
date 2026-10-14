use super::Probe;
use crate::runtime::process::{LIMIT, capture};
use std::process::{Command, Output};
use std::time::Duration;

#[derive(serde::Serialize)]
pub struct Observation {
    pub stdout: String,
    pub matches: bool,
}

impl Probe {
    pub const LIMIT: Duration = LIMIT;

    pub fn command(&self) -> Result<Command, String> {
        self.validate()?;
        let mut command = crate::config::detached(&self.argv[0]);
        command.args(&self.argv[1..]);
        Ok(command)
    }

    pub fn observe(&self, command: &mut Command, limit: Duration) -> Result<Observation, String> {
        self.validate()?;
        let actual = std::iter::once(command.get_program()).chain(command.get_args());
        if !actual.eq(self.argv.iter().map(std::ffi::OsStr::new)) {
            return Err("probe command differs from its rule".into());
        }
        let output =
            capture(command, limit).map_err(|error| format!("probe {}: {error}", self.argv[0]))?;
        self.observation(output)
    }

    pub fn run(
        &self,
        execution: &crate::config::Execution,
        limit: Duration,
    ) -> Result<Observation, String> {
        self.validate()?;
        let output = execution
            .command(&self.argv[0])
            .and_then(|mut command| capture(command.args(&self.argv[1..]), limit))
            .map_err(|error| format!("probe {}: {error}", self.argv[0]))?;
        self.observation(output)
    }

    fn observation(&self, output: Output) -> Result<Observation, String> {
        let matches = self.check(&output)?;
        Ok(Observation {
            stdout: String::from_utf8(output.stdout).map_err(|_| "probe stdout is not UTF-8")?,
            matches,
        })
    }
}
