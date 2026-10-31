use super::{Fault, Read, Result, registry::Registry, wire::ARCHIVE};
use std::{
    io::{Read as _, Seek},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Remote {
    agent: ureq::Agent,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .https_only(true)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(30)))
                .build()
                .into(),
        }
    }
}

impl Read for Remote {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        if limit > ARCHIVE {
            return Err(Fault::refused("public read budget exceeds protocol bounds"));
        }
        if let Some(path) = url.strip_prefix("https://api.github.com/") {
            return self.github(path, limit);
        }
        let response = self
            .agent
            .get(url)
            .header("User-Agent", "plumb")
            .call()
            .map_err(|_| Fault::unknown(format!("public material is unavailable: {url}")))?;
        let mut bytes = Vec::new();
        response
            .into_body()
            .as_reader()
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Fault::unknown("public material read failed"))?;
        if bytes.len() > limit {
            return Err(Fault::refused("public material exceeds bounds"));
        }
        Ok(bytes)
    }

    fn image(&self, reference: &str) -> Result<String> {
        Registry(&self.agent).inspect(reference)
    }
}

impl Remote {
    fn github(&self, path: &str, limit: usize) -> Result<Vec<u8>> {
        let mut output =
            tempfile::tempfile().map_err(|_| Fault::unknown("cannot hold metadata output"))?;
        let handle = output
            .try_clone()
            .map_err(|_| Fault::unknown("cannot hand off metadata output"))?;
        let mut child = Command::new("gh")
            .args(["api", "--hostname", "github.com", path])
            .stdin(Stdio::null())
            .stdout(handle)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Fault::unknown("cannot start GitHub metadata reader"))?;
        let started = Instant::now();
        let status = loop {
            let size = match output.metadata() {
                Ok(value) => value.len(),
                Err(_) => return Err(Self::stop(&mut child, "metadata output is unreadable")),
            };
            if started.elapsed() > Duration::from_secs(30) || size > limit as u64 {
                return Err(Self::stop(
                    &mut child,
                    "metadata read exceeded its time or output budget",
                ));
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => return Err(Self::stop(&mut child, "metadata reader did not settle")),
            }
        };
        if !status.success() {
            return Err(Fault::unknown(
                "GitHub metadata is unavailable; authenticate gh for read access",
            ));
        }
        output
            .rewind()
            .map_err(|_| Fault::unknown("cannot rewind metadata output"))?;
        let mut bytes = Vec::new();
        output
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Fault::unknown("cannot read metadata output"))?;
        if bytes.len() > limit {
            return Err(Fault::refused("metadata output exceeds bounds"));
        }
        Ok(bytes)
    }

    fn stop(child: &mut Child, reason: &str) -> Fault {
        let _ = child.kill();
        let _ = child.wait();
        Fault::unknown(reason)
    }
}
