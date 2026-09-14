use runseal::tool::{
    Reply,
    cloudflare::{api::Fault, invoke},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[path = "bucket.rs"]
mod bucket;

#[path = "policy.rs"]
mod policy;
pub use policy::Policy;

pub use bucket::{Bucket, Custom};

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
    pub resource: Resource,
    pub expires: String,
}

pub enum Resource {
    Exact(String),
    Set {
        account: String,
        buckets: Vec<String>,
    },
}

impl Resource {
    pub fn policy(&self) -> Value {
        match self {
            Self::Exact(resource) => json!({ resource.clone(): "*" }),
            Self::Set { account, buckets } => Value::Object(
                buckets
                    .iter()
                    .map(|bucket| {
                        (
                            format!("com.cloudflare.edge.r2.bucket.{account}_default_{bucket}"),
                            Value::String("*".into()),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

pub struct Factory {
    account: String,
    api: String,
    token: String,
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
        let resources = grant.resource.policy();
        let mut body = json!({
            "name": grant.name,
            "expires_on": grant.expires,
            "policies": [{
                "effect": "allow",
                "resources": resources,
                "permission_groups": [{ "id": grant.permission }],
            }],
        });
        if grant.expires.is_empty() {
            body.as_object_mut()
                .expect("token body is an object")
                .remove("expires_on");
        }
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

    fn invoke(&self, token: &str, args: &[&str], body: Option<Value>) -> Result<Reply, Failure> {
        call(
            Authority {
                account: &self.account,
                api: &self.api,
                token,
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

pub(super) fn field(value: &Value, name: &str) -> Result<String, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("cloudflare record has no {name}"))
}

pub(super) fn status(value: &Value, name: &str) -> String {
    value
        .get("status")
        .and_then(Value::as_object)
        .and_then(|held| held.get(name))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) type Failure = (Option<u16>, String);

pub(super) fn detail(failure: Failure) -> String {
    failure.1
}
