use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::process::{Command, ExitStatus};

pub(super) struct Output {
    pub stdout: File,
    stderr: File,
}

impl Output {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            stdout: tempfile::tempfile().map_err(|error| error.to_string())?,
            stderr: tempfile::tempfile().map_err(|error| error.to_string())?,
        })
    }

    pub fn execute(&self, mut command: Command) -> Result<ExitStatus, String> {
        super::super::signal::check()?;
        command.stdout(self.stdout.try_clone().map_err(|error| error.to_string())?);
        command.stderr(self.stderr.try_clone().map_err(|error| error.to_string())?);
        isolate(&mut command);
        let child = command
            .spawn()
            .map_err(|error| format!("cannot start test command: {error}"))?;
        let mut child = Running(Some(child));
        let status = child
            .0
            .as_mut()
            .expect("owned test child")
            .wait()
            .map_err(|error| format!("cannot reap test command: {error}"))?;
        child.0 = None;
        crate::diagnostic::event(
            "command.exit",
            || serde_json::json!({"code":status.code(),"success":status.success()}),
        );
        Ok(status)
    }

    pub fn text(&self) -> Result<String, String> {
        use std::io::Read;
        let mut file = self.stdout.try_clone().map_err(|error| error.to_string())?;
        file.rewind().map_err(|error| error.to_string())?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|error| error.to_string())?;
        Ok(text)
    }

    pub fn render(&self, stdout: bool) -> Result<(), String> {
        let mut sink = std::io::stderr().lock();
        for source in [stdout.then_some(&self.stdout), Some(&self.stderr)]
            .into_iter()
            .flatten()
        {
            let mut source = source.try_clone().map_err(|error| error.to_string())?;
            source
                .seek(SeekFrom::Start(0))
                .map_err(|error| error.to_string())?;
            std::io::copy(&mut source, &mut sink).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

struct Running(Option<std::process::Child>);

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            while let Err(error) = child.wait() {
                eprintln!(
                    "plumb guard: test command cleanup remains unknown; retaining execution: {error}"
                );
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}

#[cfg(unix)]
fn isolate(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn isolate(_: &mut Command) {}
