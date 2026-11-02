use super::{Fault, Read, Result, registry::Registry, wire::ARCHIVE};
use sha2::{Digest, Sha256};
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
        let mut command = Command::new("gh");
        command.args(["api", "--hostname", "github.com", path]);
        Metadata::new(path).read(&mut command, limit, Duration::from_secs(30))
    }
}

struct Metadata {
    purpose: &'static str,
    request: String,
}

impl Metadata {
    fn new(path: &str) -> Self {
        let route: Vec<_> = path
            .split('?')
            .next()
            .unwrap_or_default()
            .split('/')
            .take(10)
            .collect();
        let purpose = match route.as_slice() {
            ["repos", _, _] => "repository",
            ["repos", _, _, "branches", _] => "branch",
            ["repos", _, _, "git", "commits", _] => "commit",
            ["repos", _, _, "git", "ref", "tags", ..] => "tag reference",
            ["repos", _, _, "git", "tags", _] => "tag object",
            ["repos", _, _, "contents", ..] => "source content",
            ["repos", _, _, "actions", "runs", _, "attempts", _, "jobs"] => "attempt jobs",
            ["repos", _, _, "actions", "runs", _, "attempts", _] => "run attempt",
            _ => "request",
        };
        Self {
            purpose,
            request: format!("{:x}", Sha256::digest(path.as_bytes())),
        }
    }

    fn reason(&self, reason: &str) -> String {
        format!(
            "GitHub metadata {} [request sha256:{}]: {reason}",
            self.purpose, self.request
        )
    }

    fn unknown(&self, reason: &str) -> Fault {
        Fault::unknown(self.reason(reason))
    }

    fn read(&self, command: &mut Command, limit: usize, timeout: Duration) -> Result<Vec<u8>> {
        let mut output = tempfile::tempfile().map_err(|_| self.unknown("cannot hold output"))?;
        let handle = output
            .try_clone()
            .map_err(|_| self.unknown("cannot hand off output"))?;
        let mut child = command
            .stdin(Stdio::null())
            .stdout(handle)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| self.unknown(&format!("spawn failed ({:?})", error.kind())))?;
        let started = Instant::now();
        let status = loop {
            let size = match output.metadata() {
                Ok(value) => value.len(),
                Err(_) => return Err(self.stop(&mut child, "output metadata is unreadable")),
            };
            if started.elapsed() > timeout {
                return Err(self.stop(&mut child, "time budget exceeded"));
            }
            if size > limit as u64 {
                return Err(self.stop(&mut child, "output budget exceeded"));
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => return Err(self.stop(&mut child, "child did not settle")),
            }
        };
        if !status.success() {
            let exit = status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unavailable".into());
            return Err(self.unknown(&format!("child failed (exit {exit}; cause unknown)")));
        }
        self.body(&mut output, limit)
    }

    fn body(&self, mut output: impl std::io::Read + Seek, limit: usize) -> Result<Vec<u8>> {
        output
            .rewind()
            .map_err(|_| self.unknown("cannot rewind output"))?;
        let mut bytes = Vec::new();
        output
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| self.unknown("cannot read output"))?;
        if bytes.len() > limit {
            return Err(Fault::refused(self.reason("output exceeds bounds")));
        }
        Ok(bytes)
    }

    fn stop(&self, child: &mut Child, reason: &str) -> Fault {
        let _ = child.kill();
        let _ = child.wait();
        self.unknown(reason)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::windtun::qualification::State;
    use std::io::{Cursor, SeekFrom};

    fn command(body: &str) -> (tempfile::TempDir, Command) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("gh");
        crate::testing::executable(&path, body);
        let mut command = Command::new(path);
        command.current_dir(root.path());
        (root, command)
    }

    #[test]
    fn nonzero() {
        let (_root, mut child) = command("#!/bin/sh\nprintf NEVER_SHOW >&2\nexit 7\n");
        let path = "repos/PerishLab/secret/contents/token?ref=NEVER_SHOW";
        let request = Metadata::new(path);
        let fault = request
            .read(&mut child, 64, Duration::from_secs(2))
            .unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("source content"));
        assert!(
            fault
                .reason
                .contains(&format!("{:x}", Sha256::digest(path.as_bytes())))
        );
        assert!(fault.reason.contains("exit 7; cause unknown"));
        assert!(!fault.reason.contains("NEVER_SHOW"));
        assert!(!fault.reason.contains("secret"));
        assert!(!fault.reason.contains("authenticate"));
        assert!(fault.reason.len() < 240);
    }

    #[test]
    fn spawn() {
        let root = tempfile::tempdir().unwrap();
        let fault = Metadata::new("repos/PerishLab/wharf")
            .read(
                &mut Command::new(root.path().join("NEVER_SHOW")),
                64,
                Duration::from_secs(2),
            )
            .unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("repository"));
        assert!(fault.reason.contains("spawn failed (NotFound)"));
        assert!(!fault.reason.contains("NEVER_SHOW"));
    }

    #[test]
    fn time() {
        let (_root, mut child) = command("#!/bin/sh\nexec sleep 30\n");
        let request = Metadata::new("repos/PerishLab/wharf/branches/main");
        let fault = request.read(&mut child, 64, Duration::ZERO).unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("time budget exceeded"));
        let mut held = child.stdout(Stdio::null()).spawn().unwrap();
        request.stop(&mut held, "stopped");
        assert!(held.try_wait().unwrap().is_some());
    }

    #[test]
    fn output() {
        let (_root, mut child) = command("#!/bin/sh\nprintf TOO_MUCH\nexec sleep 30\n");
        let fault = Metadata::new("repos/PerishLab/wharf")
            .read(&mut child, 1, Duration::from_secs(2))
            .unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("output budget exceeded"));
        assert!(!fault.reason.contains("TOO_MUCH"));
    }

    struct Broken(bool);

    impl Seek for Broken {
        fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
            if self.0 {
                Err(std::io::Error::other("NEVER_SHOW"))
            } else {
                Ok(0)
            }
        }
    }

    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("NEVER_SHOW"))
        }
    }

    #[test]
    fn readback() {
        let request = Metadata::new("repos/PerishLab/wharf");
        for (broken, reason) in [
            (true, "cannot rewind output"),
            (false, "cannot read output"),
        ] {
            let fault = request.body(Broken(broken), 1).unwrap_err();
            assert_eq!(fault.state, State::Unknown);
            assert!(fault.reason.contains(reason));
            assert!(!fault.reason.contains("NEVER_SHOW"));
        }
        let fault = request.body(Cursor::new(b"TOO_MUCH"), 1).unwrap_err();
        assert_eq!(fault.state, State::Refused);
        assert!(!fault.reason.contains("TOO_MUCH"));
    }

    #[test]
    fn positive() {
        let (_root, mut child) = command("#!/bin/sh\nprintf '{\"ok\":true}'\n");
        let request = Metadata::new("repos/PerishLab/wharf");
        assert_eq!(
            request
                .read(&mut child, 64, Duration::from_secs(2))
                .unwrap(),
            b"{\"ok\":true}"
        );
        assert_ne!(
            request.request,
            Metadata::new("repos/PerishLab/keel").request
        );
    }
}
