use plumb::rig::Site;
use runseal::tool::cloudflare::{api::Fault, invoke};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bond {
    Yes,
    No,
    Unknown,
}

impl fmt::Display for Bond {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yes => write!(out, "yes"),
            Self::No => write!(out, "no"),
            Self::Unknown => write!(out, "unknown"),
        }
    }
}

pub struct Vantage {
    vars: BTreeMap<String, String>,
    domain: String,
}

impl Vantage {
    pub fn new(site: &Site) -> Self {
        Self {
            vars: BTreeMap::from([
                ("CLOUDFLARE_ACCOUNT_ID".into(), site.account.clone()),
                ("CLOUDFLARE_API_TOKEN".into(), site.token.clone()),
                ("CLOUDFLARE_API_URL".into(), site.api.clone()),
            ]),
            domain: site.domain.clone(),
        }
    }

    pub fn binding(&self) -> Bond {
        let Ok(listed) = self.call(&["worker", "domain", "list"]) else {
            return Bond::Unknown;
        };
        let attached = listed
            .as_array()
            .into_iter()
            .flatten()
            .any(|entry| entry.get("hostname").and_then(Value::as_str) == Some(&self.domain));
        if attached { Bond::Yes } else { Bond::No }
    }

    pub fn verify(&self) -> Result<String, String> {
        let seen = match self.call(&["token", "user", "verify"]) {
            Ok(seen) => seen,
            Err(_) => self
                .call(&["token", "account", "verify"])
                .map_err(|(_, detail)| detail)?,
        };
        let status = seen
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| "token verification returned no status".to_string())?;
        if status == "active" {
            Ok(status.to_string())
        } else {
            Err(format!("token is {status}"))
        }
    }

    pub fn worker(&self, name: &str) -> Result<bool, String> {
        match self.call(&["worker", "service", "show", name]) {
            Ok(_) => Ok(true),
            Err((Some(404), _)) => Ok(false),
            Err((_, detail)) => Err(detail),
        }
    }

    pub fn get(&self, seat: &str) -> Result<Value, String> {
        self.reach("GET", seat, None)
    }

    pub fn post(&self, seat: &str, body: &Value) -> Result<Value, String> {
        self.reach("POST", seat, Some(body))
    }

    fn reach(&self, method: &str, seat: &str, body: Option<&Value>) -> Result<Value, String> {
        let account = self
            .vars
            .get("CLOUDFLARE_ACCOUNT_ID")
            .ok_or_else(|| "no Cloudflare account is declared".to_string())?;
        let token = self
            .vars
            .get("CLOUDFLARE_API_TOKEN")
            .ok_or_else(|| "no Cloudflare token is held".to_string())?;
        let base = self
            .vars
            .get("CLOUDFLARE_API_URL")
            .map(String::as_str)
            .unwrap_or("https://api.cloudflare.com/client/v4");
        let url = format!("{}/accounts/{account}/{seat}", base.trim_end_matches('/'));
        let mut command = std::process::Command::new("curl");
        command
            .args([
                "--silent",
                "--show-error",
                "--location",
                "--request",
                method,
            ])
            .arg("--header")
            .arg(format!("Authorization: Bearer {token}"))
            .arg("--header")
            .arg("Content-Type: application/json");
        if let Some(body) = body {
            command.arg("--data").arg(body.to_string());
        }
        let output = command
            .arg(&url)
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        let held: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot parse {url}: {error}"))?;
        if held.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(format!("cloudflare refused {method} {seat}: {}", held));
        }
        Ok(held.get("result").cloned().unwrap_or(Value::Null))
    }

    fn call(&self, args: &[&str]) -> Result<Value, (Option<u16>, String)> {
        let args = args
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        invoke(&args, &self.vars, None)
            .map(|reply| reply.value)
            .map_err(|error| {
                (
                    error.downcast_ref::<Fault>().map(Fault::status),
                    error.to_string(),
                )
            })
    }
}
