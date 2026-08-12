use plumb::rig::Site;
use plumb::vendor::cloudflare::Account;
use serde_json::Value;
use std::fmt;

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
    account: Account,
    domain: String,
}

impl Vantage {
    pub fn new(site: &Site) -> Self {
        Self {
            account: Account::new(site.account.clone(), site.api.clone(), site.token.clone()),
            domain: site.domain.clone(),
        }
    }

    pub fn binding(&self) -> Bond {
        let route = format!("{}/workers/domains", self.account.scope());
        let Ok(listed) = self.account.read(&route) else {
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
        let scoped = format!("{}/tokens/verify", self.account.scope());
        let seen = match self.account.read("/user/tokens/verify") {
            Ok(seen) => seen,
            Err(_) => self.account.read(&scoped)?,
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
        let route = format!(
            "{}/workers/services/{}",
            self.account.scope(),
            segment(name)
        );
        match self.account.read(&route) {
            Ok(_) => Ok(true),
            Err(fault) if fault.status == 404 => Ok(false),
            Err(fault) => Err(fault.into()),
        }
    }
}

fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
