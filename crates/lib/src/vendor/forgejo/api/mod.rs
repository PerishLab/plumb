mod branch;
mod protection;
mod pull;
mod repository;
mod status;

use super::model::Remote;
use crate::vendor::{Response, send};
use serde_json::Value;

pub use branch::settled;

pub struct Client {
    pub(super) remote: Remote,
    token: String,
}

impl Client {
    pub fn new(remote: Remote) -> Result<Self, String> {
        let token = super::token::read(&remote)?;
        if token.contains(['\r', '\n']) {
            return Err("forgejo token contains a line break".into());
        }
        Ok(Self { remote, token })
    }

    fn bearing(&self) -> String {
        format!("token {}", self.token)
    }

    fn user(&self) -> Result<String, String> {
        let response = send(
            &format!("{}://{}/api/v1/user", self.remote.scheme, self.remote.host),
            "GET",
            None,
            Some(&self.bearing()),
        )?;
        let login = response
            .value
            .get("login")
            .and_then(Value::as_str)
            .unwrap_or("");
        if response.status == 200 && !login.is_empty() {
            Ok(login.to_string())
        } else {
            Err(failure(
                "fetching current user",
                response.status,
                &response.value,
            ))
        }
    }

    pub(super) fn request(
        &self,
        method: &str,
        route: &str,
        body: Option<Value>,
    ) -> Result<Response, String> {
        send(
            &format!(
                "{}://{}/api/v1/repos/{}/{}{}",
                self.remote.scheme, self.remote.host, self.remote.owner, self.remote.repo, route
            ),
            method,
            body,
            Some(&self.bearing()),
        )
    }

    pub(super) fn web(&self, route: &str) -> Result<Response, String> {
        send(
            &format!(
                "{}://{}/{}/{}{}",
                self.remote.scheme, self.remote.host, self.remote.owner, self.remote.repo, route
            ),
            "GET",
            None,
            Some(&self.bearing()),
        )
    }
}

pub fn public(url: &str) -> Result<Value, String> {
    crate::vendor::public(url)
}

pub fn failure(action: &str, status: u16, value: &Value) -> String {
    let detail = value
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string());
    format!("forgejo: {action} failed ({status}): {detail}")
}

fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|held| match held {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (held as char).to_string()
            }
            _ => format!("%{held:02X}"),
        })
        .collect()
}
