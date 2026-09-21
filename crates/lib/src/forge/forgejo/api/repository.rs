use super::Client;
use serde_json::Value;
use std::collections::BTreeSet;

impl Client {
    pub fn held(&self, owner: &str) -> Result<BTreeSet<String>, String> {
        let value = self.call(&["org", "secret", "list", owner])?;
        names(&value, "organization")
    }

    pub fn store(&self, owner: &str, name: &str, value: &str) -> Result<u16, String> {
        self.call(&["org", "secret", "set", owner, name, "--body", value])?;
        Ok(204)
    }

    pub fn patch(&self, route: &str, body: Value) -> Result<u16, String> {
        if !route.is_empty() {
            return Err("forgejo: repository patch route is not supported".into());
        }
        self.call(&["repo", "edit", "--body", &body.to_string()])?;
        Ok(200)
    }

    pub fn unset(&self, name: &str) -> Result<u16, String> {
        self.call(&["secret", "delete", name])?;
        Ok(204)
    }

    pub fn secrets(&self) -> Result<BTreeSet<String>, String> {
        let value = self.call(&["secret", "list"])?;
        names(&value, "repository")
    }

    pub fn set(&self, name: &str, value: &str) -> Result<u16, String> {
        self.call(&["secret", "set", name, "--body", value])?;
        Ok(204)
    }

    pub fn remove(&self) -> Result<u16, String> {
        self.call(&["repo", "delete"])?;
        Ok(204)
    }
}

fn names(value: &Value, scope: &str) -> Result<BTreeSet<String>, String> {
    let listed = value
        .as_array()
        .ok_or_else(|| format!("forgejo: {scope} secret list is not an array"))?;
    listed
        .iter()
        .map(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("forgejo: {scope} secret has no name"))
        })
        .collect()
}
