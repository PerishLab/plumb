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
