use runseal::tool::Reply;
use serde_json::Value;

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

    fn call(&self, args: &[&str]) -> Result<Reply, Failure> {
        self.send(args, None)
    }

    fn send(&self, args: &[&str], body: Option<Value>) -> Result<Reply, Failure> {
        self.factory.invoke(self.token.value(), args, body)
    }
}

impl Custom {
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
