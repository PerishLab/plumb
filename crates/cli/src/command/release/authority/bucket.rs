use runseal::tool::Reply;
use serde_json::{Value, json};

use super::{Factory, Failure, Minted, detail, field, status};

pub struct Bucket<'a> {
    factory: &'a Factory,
    token: &'a Minted,
    name: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Custom {
    pub domain: String,
    pub zone: String,
    pub enabled: bool,
    pub tls: String,
    pub ownership: String,
    pub certificate: String,
}

impl<'a> Bucket<'a> {
    pub fn new(factory: &'a Factory, token: &'a Minted, name: &'a str) -> Self {
        Self {
            factory,
            token,
            name,
        }
    }

    pub fn live(&self) -> Result<bool, String> {
        match self.call(&["r2", "bucket", "show", self.name]) {
            Ok(_) => Ok(true),
            Err((Some(404), _)) => Ok(false),
            Err(error) => Err(detail(error)),
        }
    }

    pub fn create(&self) -> Result<(), String> {
        self.call(&["r2", "bucket", "create", self.name])
            .map(|_| ())
            .map_err(detail)
    }

    pub fn custom(&self) -> Result<Vec<Custom>, String> {
        let reply = self
            .call(&["r2", "bucket", "domain", "list", self.name])
            .map_err(detail)?;
        let listed = reply
            .value
            .get("domains")
            .and_then(Value::as_array)
            .ok_or_else(|| "cloudflare returned no custom domain list".to_string())?;
        listed.iter().map(Custom::read).collect()
    }

    pub fn bind(&self, domain: &str, zone: &str) -> Result<(), String> {
        match self.find(domain)? {
            None => self.attach(domain, zone)?,
            Some(held) if held.zone != zone => {
                return Err(format!("custom domain belongs to zone {}", held.zone));
            }
            Some(held) if !held.delivery() => self.normalize(domain)?,
            Some(_) => {}
        }
        Ok(())
    }

    pub fn find(&self, domain: &str) -> Result<Option<Custom>, String> {
        let mut found = self
            .custom()?
            .into_iter()
            .filter(|held| held.domain == domain)
            .collect::<Vec<_>>();
        match found.len() {
            0 => Ok(None),
            1 => Ok(found.pop()),
            count => Err(format!(
                "cloudflare returned {count} custom domains named {domain}"
            )),
        }
    }

    pub fn detach(&self, domain: &str) -> Result<(), String> {
        self.call(&["r2", "bucket", "domain", "delete", self.name, domain])
            .map(|_| ())
            .map_err(detail)
    }

    pub fn erase(&self) -> Result<(), String> {
        self.call(&["r2", "bucket", "delete", self.name])
            .map(|_| ())
            .map_err(detail)
    }

    fn attach(&self, domain: &str, zone: &str) -> Result<(), String> {
        self.send(
            &["r2", "bucket", "domain", "create", self.name],
            Some(json!({
                "domain": domain,
                "enabled": true,
                "zoneId": zone,
                "minTLS": "1.2",
            })),
        )
        .map(|_| ())
        .map_err(detail)
    }

    fn normalize(&self, domain: &str) -> Result<(), String> {
        self.send(
            &["r2", "bucket", "domain", "edit", self.name, domain],
            Some(json!({ "enabled": true, "minTLS": "1.2" })),
        )
        .map(|_| ())
        .map_err(detail)
    }

    fn call(&self, args: &[&str]) -> Result<Reply, Failure> {
        self.send(args, None)
    }

    fn send(&self, args: &[&str], body: Option<Value>) -> Result<Reply, Failure> {
        self.factory.invoke(self.token.value(), args, body)
    }
}

impl Custom {
    pub fn ready(&self) -> bool {
        self.delivery() && self.ownership == "active" && self.certificate == "active"
    }

    fn delivery(&self) -> bool {
        self.enabled && self.tls == "1.2"
    }

    fn read(entry: &Value) -> Result<Self, String> {
        Ok(Self {
            domain: field(entry, "domain")?,
            zone: field(entry, "zoneId")?,
            enabled: entry
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            tls: entry
                .get("minTLS")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            ownership: status(entry, "ownership"),
            certificate: status(entry, "ssl"),
        })
    }
}
