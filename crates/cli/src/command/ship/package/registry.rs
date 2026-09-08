use crate::command::ship::attachment::Identity;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

pub(in crate::command::ship) fn publisher(root: &Path) -> Result<plumb::config::Execution, String> {
    let governance = crate::shape::product::resolve(root, "")?;
    let profile = governance
        .profile
        .as_ref()
        .ok_or("registry publication requires a Product Profile")?;
    let probes = crate::catalog::probe::read(&profile.manifest, ["Containerfile"])?;
    let execution = plumb::config::Execution::new(
        crate::execution::environment("registry")?,
        &["regctl".into()],
        root,
    )?;
    let mut observed = false;
    for rules in probes.values() {
        let probe = plumb::rule::Probe::select(rules, &plumb::config::platform())?;
        if probe.argv[0] != "regctl" {
            continue;
        }
        if !probe.run(&execution)?.matches {
            return Err("registry client probe disagrees with the publication contract".into());
        }
        observed = true;
    }
    if !observed {
        return Err("registry publication requires a regctl probe".into());
    }
    Ok(execution)
}

pub(in crate::command::ship) struct Session<'a> {
    seat: tempfile::TempDir,
    execution: Option<&'a plumb::config::Execution>,
}

impl<'a> Session<'a> {
    pub fn open(execution: Option<&'a plumb::config::Execution>) -> Result<Self, String> {
        let seat = tempfile::Builder::new()
            .prefix("plumb-registry-")
            .tempdir()
            .map_err(|error| format!("cannot isolate registry configuration: {error}"))?;
        let held = Self { seat, execution };
        for name in ["authenticated", "anonymous"] {
            status(held.base(name)?.args([
                "config",
                "set",
                "--docker-cred=false",
                "--docker-cert=false",
            ]))?;
        }
        Ok(held)
    }

    pub fn path(&self) -> &Path {
        self.seat.path()
    }

    pub fn command(&self) -> Result<Command, String> {
        self.base("authenticated")
    }

    pub fn anonymous(&self) -> Result<Command, String> {
        self.base("anonymous")
    }

    pub fn login(&self, registry: &str, identity: &Identity<'_>) -> Result<(), String> {
        let mut child = self
            .command()?
            .args([
                "registry",
                "login",
                registry,
                "--user",
                identity.user,
                "--pass-stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("cannot run registry login: {error}"))?;
        let written = child
            .stdin
            .take()
            .ok_or_else(|| "registry login refused stdin".to_string())
            .and_then(|mut input| {
                input
                    .write_all(identity.token.as_bytes())
                    .map_err(|error| format!("cannot send registry credential: {error}"))
            });
        let status = child
            .wait()
            .map_err(|error| format!("cannot wait for registry login: {error}"))?;
        written?;
        if !status.success() {
            return Err("image attachment login failed".into());
        }
        Ok(())
    }

    fn base(&self, name: &str) -> Result<Command, String> {
        let mut command = self.execution.map_or_else(
            || Ok(Command::new("regctl")),
            |execution| execution.command("regctl"),
        )?;
        command
            .env("REGCTL_CONFIG", self.seat.path().join(name))
            .env_remove("DOCKER_AUTH_CONFIG");
        Ok(command)
    }
}

pub(super) fn output(command: &mut Command) -> Result<String, String> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot run registry client: {error}"))?;
    if !output.status.success() {
        return Err("registry client operation failed".into());
    }
    String::from_utf8(output.stdout).map_err(|_| "registry output is not UTF-8".into())
}

pub(super) fn status(command: &mut Command) -> Result<(), String> {
    if command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("cannot run registry client: {error}"))?
        .success()
    {
        Ok(())
    } else {
        Err("registry client operation failed".into())
    }
}
