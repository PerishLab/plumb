mod authority;
mod branch;
mod protection;
mod pull;
mod repository;
mod status;

use super::model::Remote;
use serde_json::Value;
use std::collections::BTreeMap;

pub use authority::{Minted, Token, scopes};
pub use branch::settled;

pub struct Client {
    pub(super) remote: Remote,
    vars: BTreeMap<String, String>,
}

impl Client {
    pub fn remote(&self) -> &Remote {
        &self.remote
    }

    pub fn new(remote: Remote) -> Result<Self, String> {
        let url = format!("{}://{}", remote.scheme, remote.host);
        let vars = super::settings::vars(url)?;
        Ok(Self { remote, vars })
    }

    pub(super) fn call(&self, args: &[&str]) -> Result<Value, String> {
        self.reply(args).map(|reply| reply.value)
    }

    pub(super) fn reply(&self, args: &[&str]) -> Result<runseal::tool::Reply, String> {
        let mut argv = vec![
            "--repo".to_string(),
            format!("{}/{}", self.remote.owner, self.remote.repo),
        ];
        argv.extend(args.iter().map(|arg| arg.to_string()));
        runseal::tool::call("forgejo", &argv, &self.vars).map_err(|error| {
            format!(
                "forgejo refused {} on {}/{}: {error}",
                attempted(args),
                self.remote.owner,
                self.remote.repo
            )
        })
    }

    pub fn account(&self) -> Result<String, String> {
        self.user()
    }

    fn user(&self) -> Result<String, String> {
        let value = self.call(&["user", "show"])?;
        let login = value.get("login").and_then(Value::as_str).unwrap_or("");
        if !login.is_empty() {
            Ok(login.to_string())
        } else {
            Err("forgejo: current user response has no login".into())
        }
    }
}

fn attempted(args: &[&str]) -> String {
    let named = args
        .iter()
        .take_while(|arg| !arg.starts_with("--"))
        .copied()
        .collect::<Vec<_>>();
    if named.is_empty() {
        "an operation".to_string()
    } else {
        named.join(" ")
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
