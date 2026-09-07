use super::execution::Execution;
use std::path::Path;
use std::process::Command;

pub struct Cache<'a> {
    execution: &'a Execution,
    directory: tempfile::TempDir,
    port: u16,
    active: bool,
}

impl<'a> Cache<'a> {
    pub fn start(execution: &'a Execution, directory: &Path) -> Result<Option<Self>, String> {
        if execution
            .environment
            .tools
            .get("RUSTC_WRAPPER")
            .map(String::as_str)
            != Some("sccache")
        {
            return Ok(None);
        }
        if execution.environment.get("CARGO_INCREMENTAL") != Some("0") {
            return Err("sccache requires bound CARGO_INCREMENTAL=0".into());
        }
        if execution
            .environment
            .evidence()
            .keys()
            .any(|key| key.starts_with("SCCACHE_") && *key != "SCCACHE_DIR")
        {
            return Err("compiler cache refuses settings outside its owned configuration".into());
        }
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create compiler cache: {error}"))?;
        let directory = directory
            .canonicalize()
            .map_err(|error| format!("cannot resolve compiler cache: {error}"))?;
        let configuration = toml::to_string(&serde_json::json!({
            "server_startup_timeout_ms": 4000,
            "cache": {"disk": {"dir": directory.to_str().ok_or("compiler cache path is not UTF-8")?}},
        }))
        .map_err(|error| format!("cannot encode compiler cache configuration: {error}"))?;
        let temporary = tempfile::tempdir()
            .map_err(|error| format!("cannot reserve compiler cache service: {error}"))?;
        std::fs::write(temporary.path().join("config"), configuration)
            .map_err(|error| format!("cannot write compiler cache configuration: {error}"))?;
        let mut held = Self {
            execution,
            directory: temporary,
            port: 0,
            active: false,
        };
        let output = held.control("--start-server")?;
        let text = std::str::from_utf8(&output)
            .map_err(|_| "compiler cache startup output is not UTF-8")?;
        let ports = text
            .lines()
            .filter_map(|line| line.strip_prefix("sccache: Listening on address 127.0.0.1:"))
            .collect::<Vec<_>>();
        if ports.len() != 1 {
            return Err("compiler cache did not report one owned loopback endpoint".into());
        }
        held.port = ports[0]
            .parse()
            .ok()
            .filter(|port| *port != 0)
            .ok_or("compiler cache reported an invalid port")?;
        held.active = true;
        Ok(Some(held))
    }

    pub fn apply(&self, command: &mut Command) {
        command
            .env("SCCACHE_CONF", self.directory.path().join("config"))
            .env("SCCACHE_CACHED_CONF", self.directory.path().join("cached"))
            .env("SCCACHE_SERVER_PORT", self.port.to_string())
            .env("SCCACHE_IDLE_TIMEOUT", "30");
    }

    pub fn finish(mut self) -> Result<(), String> {
        self.control("--stop-server")?;
        self.active = false;
        Ok(())
    }

    fn control(&self, verb: &str) -> Result<Vec<u8>, String> {
        let mut command = self.execution.command("sccache")?;
        self.apply(&mut command);
        command.arg(verb);
        let output = super::process::capture(&mut command)?;
        if !output.status.success() {
            return Err(format!("compiler cache {verb} failed"));
        }
        Ok(output.stdout)
    }
}

impl Drop for Cache<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.control("--stop-server");
        }
    }
}
