use super::{Client, failure, segment};
use crate::vendor::forgejo::model::Cut;
use serde_json::{Value, json};

impl Client {
    pub fn branch(&self, name: &str) -> Result<Option<Value>, String> {
        let response = self.request("GET", &format!("/branches/{}", segment(name)), None)?;
        match response.status {
            200 if response.value.is_object() => Ok(Some(response.value)),
            404 => Ok(None),
            status => Err(failure("fetching branch", status, &response.value)),
        }
    }

    pub fn create(&self, name: &str, from: &str) -> Result<Cut, String> {
        if let Some(held) = self.branch(name)? {
            let wanted = self.branch(from)?.as_ref().map(commit).unwrap_or_default();
            return settled(name, from, &commit(&held), &wanted);
        }
        let response = self.request(
            "POST",
            "/branches",
            Some(json!({"new_branch_name": name, "old_branch_name": from})),
        )?;
        if response.status == 201 {
            Ok(Cut::Made)
        } else {
            Err(failure(
                "creating release branch",
                response.status,
                &response.value,
            ))
        }
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
    Ok(Cut::Held)
}

fn commit(value: &Value) -> String {
    value
        .pointer("/commit/id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
