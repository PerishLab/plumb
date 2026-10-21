use plumb::config::{Binding, Contract, Environment, Execution};
use std::path::Path;

pub(crate) struct Context {
    pub image: String,
}

impl Context {
    pub fn read(root: &Path) -> Result<Option<Self>, String> {
        let Some(container) = plumb::config::value("PLUMB_GUARD_CONTAINER") else {
            return Ok(None);
        };
        if !cfg!(target_os = "linux") || !hex(&container) {
            return Err(
                "PLUMB_GUARD_CONTAINER requires one exact Linux Docker container ID".into(),
            );
        }
        let environment = plumb::config::contract("probe")?.capture(std::env::vars_os())?;
        let execution = Execution::new(environment, &["docker".into()], root)?;
        let output = execution.output(&[
            "docker".into(),
            "--host=unix:///var/run/docker.sock".into(),
            "container".into(),
            "inspect".into(),
            "--format={{.Id}}\n{{.Image}}\n{{.Config.Hostname}}".into(),
            container.clone(),
        ])?;
        if !output.status.success() {
            return Err(
                "cannot inspect the declared Guard container on the local Docker socket".into(),
            );
        }
        let text = std::str::from_utf8(&output.stdout)
            .map_err(|_| "Guard container metadata is not text")?;
        let lines = text.lines().collect::<Vec<_>>();
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map_err(|error| format!("cannot identify the current Guard container: {error}"))?;
        if lines.len() != 3 || lines[0] != container || lines[2] != hostname.trim() {
            return Err("declared Guard container does not match the current container".into());
        }
        if !lines[1].strip_prefix("sha256:").is_some_and(hex) {
            return Err("Guard container did not report an immutable image ID".into());
        }
        Ok(Some(Self {
            image: lines[1].into(),
        }))
    }

    pub fn bind(&self, environment: Environment) -> Result<Environment, String> {
        environment.combine(
            Contract {
                inherit: Vec::new(),
                managed: Vec::new(),
                reject: Vec::new(),
                bind: [(
                    "PLUMB_GUARD_IMAGE".into(),
                    Binding::Value(self.image.clone()),
                )]
                .into(),
            }
            .capture([])?,
        )
    }
}

pub(super) fn hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
