use super::{Client, failure};
use crate::vendor::forgejo::model::State;
use serde_json::Value;

impl Client {
    pub fn combined(&self, commit: &str) -> Result<State, String> {
        Ok(State::read(&self.statuses(commit)?))
    }

    pub fn context(&self, commit: &str, context: &str) -> Result<State, String> {
        let response = self.statuses(commit)?;
        let mut found = response
            .get("statuses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|status| status.get("context").and_then(Value::as_str) == Some(context))
            .collect::<Vec<_>>();
        found.sort_by_key(|status| {
            status
                .get("updated_at")
                .or_else(|| status.get("created_at"))
                .and_then(Value::as_str)
                .unwrap_or("")
        });
        Ok(State {
            state: found
                .last()
                .and_then(|status| status.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            count: found.len(),
        })
    }

    fn statuses(&self, commit: &str) -> Result<Value, String> {
        let route = format!("/commits/{commit}/status");
        let response = self.request("GET", &route, None)?;
        if response.status != 200 {
            return Err(failure(
                "fetching commit status",
                response.status,
                &response.value,
            ));
        }
        Ok(response.value)
    }
}
