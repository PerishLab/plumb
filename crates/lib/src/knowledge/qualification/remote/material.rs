use super::super::{Fault, Result};
use sha2::{Digest, Sha256};
use std::io::Read;

pub(super) struct Material<'a> {
    url: &'a str,
    purpose: &'static str,
    identity: String,
}

impl<'a> Material<'a> {
    pub(super) fn new(url: &'a str) -> Self {
        let route = url.split(['?', '#']).next().unwrap_or_default();
        let parts = route.split('/').take(14).collect::<Vec<_>>();
        let purpose = match parts.as_slice() {
            ["https:", "", _, "v1", "channels", _] => "channel pointer",
            ["https:", "", _, "v1", "releases", _, _, "seal.json"] => "release seal",
            ["https:", "", _, "v1", "releases", _, _, "distribution.json"] => "distribution",
            ["https:", "", _, "v1", "objects", "sha256", _, _] => "content object",
            [
                "https:",
                "",
                _,
                "v1",
                "releases",
                _,
                _,
                "windtun",
                "latest.json",
            ] => "latest attempt",
            [
                "https:",
                "",
                _,
                "v1",
                "releases",
                _,
                _,
                "windtun",
                "attempts",
                _,
                _,
            ] => "trial record",
            ["https:", "", _, "channels", _, "scenes", "latest.json"] => "scene pointer",
            [
                "https:",
                "",
                _,
                "channels",
                _,
                "scenes",
                "versions",
                _,
                "latest.json",
            ] => "scene standing",
            [
                "https:",
                "",
                _,
                "channels",
                _,
                "scenes",
                "versions",
                _,
                "generations",
                _,
                "manifest.json",
            ] => "scene manifest",
            [
                "https:",
                "",
                _,
                "channels",
                _,
                "scenes",
                "versions",
                _,
                "generations",
                _,
                "objects",
                ..,
            ] => "scene object",
            _ => "request",
        };
        Self {
            url,
            purpose,
            identity: format!("{:x}", Sha256::digest(url.as_bytes())),
        }
    }

    pub(super) fn read(&self, agent: &ureq::Agent, limit: usize) -> Result<Vec<u8>> {
        let response = agent
            .get(self.url)
            .header("User-Agent", "plumb")
            .call()
            .map_err(|error| self.failure(error))?;
        self.body(response.into_body().as_reader(), limit)
    }

    fn failure(&self, error: ureq::Error) -> Fault {
        let cause = match error {
            ureq::Error::StatusCode(code) => format!("HTTP {code}"),
            ureq::Error::Timeout(_) => "time budget exceeded".into(),
            _ => "cause unknown".into(),
        };
        self.unknown(&format!("request failed ({cause})"))
    }

    fn body(&self, reader: impl Read, limit: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        reader
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| self.interruption(error))?;
        if bytes.len() > limit {
            return Err(Fault::refused(self.reason("output budget exceeded")));
        }
        Ok(bytes)
    }

    fn interruption(&self, error: std::io::Error) -> Fault {
        let cause = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<ureq::Error>());
        if matches!(cause, Some(ureq::Error::Timeout(_))) {
            return self.unknown("body read failed (time budget exceeded)");
        }
        self.unknown(&format!(
            "body read failed ({:?}; cause unknown)",
            error.kind()
        ))
    }

    fn reason(&self, reason: &str) -> String {
        format!(
            "public material {} [request sha256:{}]: {reason}",
            self.purpose, self.identity
        )
    }

    fn unknown(&self, reason: &str) -> Fault {
        Fault::unknown(self.reason(reason))
    }
}

#[cfg(test)]
#[path = "testing.rs"]
mod tests;
