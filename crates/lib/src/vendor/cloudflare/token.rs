use super::api::{Account, Fault};
use serde_json::{Value, json};

pub struct Held {
    pub id: String,
    pub name: String,
}

pub struct Minted {
    pub id: String,
    pub value: String,
}

pub struct Grant {
    pub name: String,
    pub permission: String,
    pub resource: String,
    pub expires: String,
}

pub struct Factory {
    account: Account,
}

impl Factory {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    pub fn id(&self) -> &str {
        &self.account.id
    }

    pub fn bearing(&self, token: &str) -> Account {
        self.account.swap(token)
    }

    pub fn verify(&self) -> Result<(), Fault> {
        let route = format!("{}/tokens/verify", self.account.scope());
        self.account.read(&route).map(|_| ())
    }

    pub fn held(&self) -> Result<Vec<Held>, String> {
        let route = format!("{}/tokens?per_page=50", self.account.scope());
        let listed = self.account.read(&route)?;
        let entries = listed
            .as_array()
            .ok_or_else(|| "cloudflare returned no token list".to_string())?;
        entries
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
        let route = format!("{}/tokens/permission_groups", self.account.scope());
        let listed = self.account.read(&route)?;
        let entries = listed
            .as_array()
            .ok_or_else(|| "cloudflare returned no permission groups".to_string())?;
        let found = entries
            .iter()
            .filter(|entry| named(entry, name) && scoped(entry, scope))
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return Err(format!(
                "expected exactly one cloudflare permission group {name}, saw {}",
                found.len()
            ));
        }
        field(found[0], "id")
    }

    pub fn create(&self, grant: &Grant) -> Result<Minted, String> {
        let route = format!("{}/tokens", self.account.scope());
        let body = json!({
            "name": grant.name,
            "expires_on": grant.expires,
            "policies": [{
                "effect": "allow",
                "resources": { grant.resource.clone(): "*" },
                "permission_groups": [{ "id": grant.permission }],
            }],
        });
        let held = self.account.result("POST", &route, Some(body))?;
        Ok(Minted {
            id: field(&held, "id")?,
            value: field(&held, "value")?,
        })
    }

    pub fn revoke(&self, id: &str) -> Result<(), Fault> {
        let route = format!("{}/tokens/{id}", self.account.scope());
        self.account.erase(&route).map(|_| ())
    }
}

fn named(entry: &Value, name: &str) -> bool {
    entry.get("name").and_then(Value::as_str) == Some(name)
}

fn scoped(entry: &Value, scope: &str) -> bool {
    entry
        .get("scopes")
        .and_then(Value::as_array)
        .is_some_and(|held| held.iter().any(|value| value.as_str() == Some(scope)))
}

fn field(entry: &Value, name: &str) -> Result<String, String> {
    entry
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("cloudflare token record has no {name}"))
}
