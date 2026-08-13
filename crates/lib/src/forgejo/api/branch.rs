use super::Client;
use crate::forgejo::model::Cut;
use serde_json::Value;

impl Client {
    pub fn branch(&self, name: &str) -> Result<Option<Value>, String> {
        match self.call(&["branch", "show", name]) {
            Ok(value) if value.is_object() => Ok(Some(value)),
            Err(error) if absent(&error) => Ok(None),
            Ok(_) => Err("forgejo: branch response is not an object".into()),
            Err(error) => Err(error),
        }
    }

    pub fn create(&self, name: &str, from: &str) -> Result<Cut, String> {
        if let Some(held) = self.branch(name)? {
            let wanted = self.branch(from)?.as_ref().map(commit).unwrap_or_default();
            return settled(name, from, &commit(&held), &wanted);
        }
        self.call(&["branch", "create", name, "--from", from])?;
        Ok(Cut::Made)
    }
}

fn absent(error: &str) -> bool {
    error.contains("404") || error.contains("missing") || error.contains("not found")
}

pub fn settled(name: &str, from: &str, seen: &str, wanted: &str) -> Result<Cut, String> {
    if seen.is_empty() || wanted.is_empty() {
        return Err(format!(
            "{name} already exists and its commit could not be compared with {from}"
        ));
    }
    if seen != wanted {
        return Err(format!(
            "{name} already exists at {seen} and {from} is {wanted}; a release line is frozen and prepare does not move it"
        ));
    }
    Ok(Cut::Held)
}

fn commit(value: &Value) -> String {
    value
        .pointer("/commit/id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
