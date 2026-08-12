use super::{Client, failure};
use crate::vendor::forgejo::model::{Pull, Strategy};
use serde_json::{Value, json};

impl Client {
    pub fn find(&self, head: &str) -> Result<Option<Pull>, String> {
        self.opened("main", head)
    }

    pub fn opened(&self, base: &str, head: &str) -> Result<Option<Pull>, String> {
        let response = self.request("GET", "/pulls?state=open&limit=50", None)?;
        if response.status != 200 {
            return Err(failure("listing pulls", response.status, &response.value));
        }
        Ok(response
            .value
            .as_array()
            .into_iter()
            .flatten()
            .find(|held| {
                held.pointer("/head/ref").and_then(Value::as_str) == Some(head)
                    && held.pointer("/base/ref").and_then(Value::as_str) == Some(base)
            })
            .and_then(number))
    }

    pub fn pull(&self, head: &str, version: &str, body: &str) -> Result<Pull, String> {
        self.raise("main", head, &format!("Packport {version}"), body)
    }

    pub fn raise(&self, base: &str, head: &str, title: &str, body: &str) -> Result<Pull, String> {
        let response = self.request(
            "POST",
            "/pulls",
            Some(json!({
                "base": base,
                "head": head,
                "title": title,
                "body": body
            })),
        )?;
        if response.status == 201 {
            number(&response.value).ok_or_else(|| "created pull has no number".to_string())
        } else {
            Err(failure("creating pull", response.status, &response.value))
        }
    }

    pub fn merge(&self, pull: u64, head: &str) -> Result<(), String> {
        self.settle(pull, head, Strategy::Merge)
    }

    pub fn settle(&self, pull: u64, head: &str, strategy: Strategy) -> Result<(), String> {
        let body = json!({
            "Do": strategy.wire(),
            "delete_branch_after_merge": false,
            "head_commit_id": head
        });
        let mut last = String::new();
        for turn in 1..=6 {
            let response =
                self.request("POST", &format!("/pulls/{pull}/merge"), Some(body.clone()))?;
            if [200, 204].contains(&response.status) {
                return Ok(());
            }
            last = failure("merging pull", response.status, &response.value);
            if ![405, 409].contains(&response.status) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(turn));
        }
        Err(last)
    }
}

fn number(value: &Value) -> Option<Pull> {
    value
        .get("number")
        .and_then(Value::as_u64)
        .map(|number| Pull {
            number,
            url: value
                .get("html_url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
}
