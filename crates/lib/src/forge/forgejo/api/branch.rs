use super::Client;
use crate::forgejo::model::Cut;
use serde_json::Value;

impl Client {
    pub fn branch(&self, name: &str) -> Result<Option<Value>, String> {
        match self.call(&["branch", "show", name]) {
            Ok(value) if value.is_object() => Ok(Some(value)),
            Ok(_) => Err("forgejo: branch response is not an object".into()),
            Err(_) => Ok(None),
        }
    }

    pub fn create(&self, name: &str, from: &str) -> Result<Cut, String> {
        if let Some(held) = self.branch(name)? {
            let wanted = self.branch(from)?.as_ref().map(commit).unwrap_or_default();
            return settled(name, from, &commit(&held), &wanted);
        }
        let made = self.call(&["branch", "create", name, "--from", from])?;
        let cut = commit(&made);
        if cut.is_empty() {
            return Err(format!("{name} was cut but exposes no head commit"));
        }
        Ok(Cut::Made(cut))
    }
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
    Ok(Cut::Held(seen.to_string()))
}

fn commit(value: &Value) -> String {
    value
        .pointer("/commit/id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
