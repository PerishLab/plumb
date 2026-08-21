use runseal::tool::{
    Reply,
    cloudflare::{api::Fault, invoke},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Held {
    pub id: String,
    pub name: String,
}

pub struct Minted {
    pub id: String,
    reply: Reply,
}

pub struct Grant {
    pub name: String,
    pub permission: String,
    pub resource: String,
    pub expires: String,
}

pub struct Factory {
    account: String,
    api: String,
    token: String,
}

pub struct Bucket<'a> {
    factory: &'a Factory,
    token: &'a Minted,
    name: &'a str,
}

pub struct Custom {
    pub domain: String,
    pub zone: String,
}

struct Authority<'a> {
    account: &'a str,
    api: &'a str,
    token: &'a str,
}

impl Factory {
    pub fn new(account: String, api: String, token: String) -> Self {
        Self {
            account,
            api,
            token,
        }
    }

    pub fn id(&self) -> &str {
        &self.account
    }

    pub fn verify(&self) -> Result<(), String> {
        self.call(&["token", "account", "verify"], None)
            .map(|_| ())
            .map_err(detail)
    }

    pub fn held(&self) -> Result<Vec<Held>, String> {
        let reply = self
            .call(&["token", "account", "list"], None)
            .map_err(detail)?;
        array(&reply.value, "token list")?
            .iter()
            .map(|entry| {
                Ok(Held {
                    id: field(entry, "id")?,
                    name: field(entry, "name")?,
                })
            })
            .collect()
    }

    pub fn permission(&self, name: &str, scope: &str) -> Result<String, String> {
        let reply = self
            .call(
                &[
                    "token",
                    "account",
                    "permission",
                    "list",
                    "--name",
                    name,
                    "--scope",
                    scope,
                ],
                None,
            )
            .map_err(detail)?;
        let found = array(&reply.value, "permission list")?;
        if found.len() != 1 {
            return Err(format!(
                "expected exactly one cloudflare permission group {name}, saw {}",
                found.len()
            ));
        }
        field(&found[0], "id")
    }

    pub fn create(&self, grant: &Grant) -> Result<Minted, String> {
        let body = json!({
            "name": grant.name,
            "expires_on": grant.expires,
            "policies": [{
                "effect": "allow",
                "resources": { grant.resource.clone(): "*" },
                "permission_groups": [{ "id": grant.permission }],
            }],
        });
        let reply = self
            .call(&["token", "account", "create"], Some(body))
            .map_err(detail)?;
        let id = field(&reply.value, "id")?;
        if reply.secret().is_none() {
            return Err("cloudflare token result has no value".into());
        }
        Ok(Minted { id, reply })
    }

    pub fn revoke(&self, id: &str) -> Result<(), String> {
        self.call(&["token", "account", "delete", id], None)
            .map(|_| ())
            .map_err(detail)
    }

    fn call(&self, args: &[&str], body: Option<Value>) -> Result<Reply, Failure> {
        call(
            Authority {
                account: &self.account,
                api: &self.api,
                token: &self.token,
            },
            args,
            body,
        )
    }
}

impl Minted {
    pub fn value(&self) -> &str {
        self.reply
            .secret()
            .expect("minted token is guarded")
            .expose()
    }
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
        listed
            .iter()
            .map(|entry| {
                Ok(Custom {
                    domain: field(entry, "domain")?,
                    zone: field(entry, "zoneId")?,
                })
            })
            .collect()
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
        call(
            Authority {
                account: &self.factory.account,
                api: &self.factory.api,
                token: self.token.value(),
            },
            args,
            None,
        )
    }
}

fn call(authority: Authority<'_>, args: &[&str], body: Option<Value>) -> Result<Reply, Failure> {
    let vars = BTreeMap::from([
        (
            "CLOUDFLARE_ACCOUNT_ID".into(),
            authority.account.to_string(),
        ),
        ("CLOUDFLARE_API_TOKEN".into(), authority.token.to_string()),
        ("CLOUDFLARE_API_URL".into(), authority.api.to_string()),
    ]);
    let args = args
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    invoke(&args, &vars, body).map_err(|error| {
        (
            error.downcast_ref::<Fault>().map(Fault::status),
            error.to_string(),
        )
    })
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("cloudflare returned no {name}"))
}

fn field(value: &Value, name: &str) -> Result<String, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("cloudflare record has no {name}"))
}

type Failure = (Option<u16>, String);

fn detail(failure: Failure) -> String {
    failure.1
}
