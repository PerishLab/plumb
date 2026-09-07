use crate::command::ship::attachment::Identity;
use std::io::{Seek as _, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};

pub(in crate::command::ship) struct Session {
    seat: tempfile::TempDir,
}

impl Session {
    pub fn open(root: &Path, registry: &str) -> Result<Self, String> {
        let seat = tempfile::Builder::new()
            .prefix("plumb-docker-")
            .tempdir()
            .map_err(|error| format!("cannot isolate Docker configuration: {error}"))?;
        let body = serde_json::json!({ "auths": { registry: {} } }).to_string();
        for name in ["authenticated", "anonymous"] {
            let path = seat.path().join(name);
            std::fs::create_dir(&path)
                .map_err(|error| format!("cannot stage Docker configuration: {error}"))?;
            std::fs::write(path.join("config.json"), &body)
                .map_err(|error| format!("cannot stage Docker authentication policy: {error}"))?;
        }
        let held = Self { seat };
        let context = output(
            Command::new("docker")
                .current_dir(root)
                .args(["context", "show"]),
        )?;
        let identity = output(
            Command::new("docker")
                .current_dir(root)
                .args(["info", "--format", "{{.ID}}"]),
        )?;
        let mut archive = tempfile::tempfile()
            .map_err(|error| format!("cannot stage Docker context: {error}"))?;
        status(
            Command::new("docker")
                .current_dir(root)
                .args(["context", "export", &context, "-"])
                .stdout(archive.try_clone().map_err(|error| error.to_string())?),
        )?;
        archive.rewind().map_err(|error| error.to_string())?;
        status(
            held.base("authenticated")
                .current_dir(root)
                .args(["context", "import", "plumb", "-"])
                .stdin(archive)
                .stdout(Stdio::null()),
        )?;
        let isolated = output(
            held.command()
                .current_dir(root)
                .args(["info", "--format", "{{.ID}}"]),
        )?;
        if identity != isolated {
            return Err("isolated Docker context selects a different daemon".into());
        }
        Ok(held)
    }

    pub fn command(&self) -> Command {
        let mut command = self.base("authenticated");
        command.args(["--context", "plumb"]);
        command
    }

    pub fn anonymous(&self) -> Command {
        let mut command = self.base("anonymous");
        command.args(["--context", "default"]);
        command
    }

    pub fn login(&self, registry: &str, identity: &Identity<'_>) -> Result<(), String> {
        let mut child = self
            .command()
            .args([
                "login",
                registry,
                "--username",
                identity.user,
                "--password-stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|error| format!("cannot run Docker login: {error}"))?;
        let written = child
            .stdin
            .take()
            .ok_or_else(|| "Docker login refused stdin".to_string())
            .and_then(|mut input| {
                input
                    .write_all(identity.token.as_bytes())
                    .map_err(|error| format!("cannot send registry credential: {error}"))
            });
        let status = child
            .wait()
            .map_err(|error| format!("cannot wait for Docker login: {error}"))?;
        written?;
        if !status.success() {
            return Err("image attachment login failed".into());
        }
        Ok(())
    }

    fn base(&self, name: &str) -> Command {
        let mut command = Command::new("docker");
        command
            .arg("--config")
            .arg(self.seat.path().join(name))
            .env_remove("DOCKER_AUTH_CONFIG");
        command
    }
}

fn output(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("cannot inspect Docker context: {error}"))?;
    if !output.status.success() {
        return Err("cannot inspect Docker context".into());
    }
    let text = String::from_utf8(output.stdout).map_err(|_| "Docker context is not UTF-8")?;
    let text = text.trim();
    if text.is_empty() || text.contains(['\n', '\r']) {
        return Err("Docker context returned no single identity".into());
    }
    Ok(text.to_string())
}

fn status(command: &mut Command) -> Result<(), String> {
    if command
        .status()
        .map_err(|error| format!("cannot isolate Docker context: {error}"))?
        .success()
    {
        Ok(())
    } else {
        Err("cannot isolate Docker context".into())
    }
}
