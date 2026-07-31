use super::api::Client;
use super::request::failure;
use serde_json::{Value, json};

impl Client {
    pub fn dispatch(
        &self,
        workflow: &str,
        reference: &str,
        inputs: Value,
    ) -> Result<Value, String> {
        let route = format!("/actions/workflows/{workflow}/dispatches");
        let response = self.request(
            "POST",
            &route,
            Some(json!({
                "ref": reference,
                "inputs": inputs,
                "return_run_info": true
            })),
        )?;
        if [200, 201, 204].contains(&response.status) {
            Ok(response.value)
        } else {
            Err(failure(
                "dispatching release workflow",
                response.status,
                &response.value,
            ))
        }
    }

    pub fn run(&self, id: u64) -> Result<Value, String> {
        let response = self.request("GET", &format!("/actions/runs/{id}"), None)?;
        if response.status == 200 && response.value.is_object() {
            Ok(response.value)
        } else {
            Err(failure(
                "fetching workflow run",
                response.status,
                &response.value,
            ))
        }
    }

    pub fn failures(&self, id: u64) -> Result<Vec<String>, String> {
        let response = self.request("GET", &format!("/actions/runs/{id}/jobs"), None)?;
        if response.status != 200 {
            return Ok(Vec::new());
        }
        Ok(response
            .value
            .get("jobs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|job| {
                ["failure", "cancelled"].iter().any(|state| {
                    job.get("status").and_then(Value::as_str) == Some(state)
                        || job.get("conclusion").and_then(Value::as_str) == Some(state)
                })
            })
            .filter_map(|job| job.get("name").and_then(Value::as_str))
            .map(str::to_string)
            .collect())
    }

    pub fn link(&self, run: &Value) -> String {
        if let Some(url) = run.get("html_url").and_then(Value::as_str)
            && !url.trim().is_empty()
        {
            return url.trim().to_string();
        }
        run.get("run_number")
            .and_then(Value::as_u64)
            .map(|number| {
                format!(
                    "{}://{}/{}/{}/actions/runs/{number}",
                    self.remote.scheme, self.remote.host, self.remote.owner, self.remote.repo
                )
            })
            .unwrap_or_default()
    }
}
