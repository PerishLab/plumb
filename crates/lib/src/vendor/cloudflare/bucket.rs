use super::api::{Account, Fault};
use serde_json::Value;

pub struct Bucket<'a> {
    account: &'a Account,
    name: String,
}

pub struct Custom {
    pub domain: String,
    pub zone: String,
}

impl<'a> Bucket<'a> {
    pub fn new(account: &'a Account, name: &str) -> Self {
        Self {
            account,
            name: name.to_string(),
        }
    }

    pub fn live(&self) -> Result<bool, String> {
        match self.account.read(&self.route()) {
            Ok(_) => Ok(true),
            Err(fault) if fault.status == 404 => Ok(false),
            Err(fault) => Err(fault.into()),
        }
    }

    pub fn custom(&self) -> Result<Vec<Custom>, String> {
        let held = self.account.read(&self.domains())?;
        let listed = held
            .get("domains")
            .and_then(Value::as_array)
            .ok_or_else(|| "cloudflare returned no custom domain list".to_string())?;
        listed
            .iter()
            .map(|entry| {
                Ok(Custom {
                    domain: text(entry, "domain")?,
                    zone: text(entry, "zoneId")?,
                })
            })
            .collect()
    }

    pub fn detach(&self, domain: &str) -> Result<(), Fault> {
        let route = format!("{}/{domain}", self.domains());
        self.account.erase(&route).map(|_| ())
    }

    pub fn erase(&self) -> Result<(), Fault> {
        self.account.erase(&self.route()).map(|_| ())
    }

    fn route(&self) -> String {
        format!("{}/r2/buckets/{}", self.account.scope(), self.name)
    }

    fn domains(&self) -> String {
        format!("{}/domains/custom", self.route())
    }
}

fn text(entry: &Value, name: &str) -> Result<String, String> {
    entry
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("cloudflare custom domain record has no {name}"))
}
