use super::Client;
use crate::forgejo::model::{Pull, Strategy};
use serde_json::Value;

impl Client {
    pub fn find(&self, head: &str) -> Result<Option<Pull>, String> {
        self.opened("main", head)
    }

    pub fn opened(&self, base: &str, head: &str) -> Result<Option<Pull>, String> {
        let value = self.call(&["pull", "list"])?;
        Ok(value
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
        let value = self.call(&[
            "pull", "create", "--base", base, "--head", head, "--title", title, "--body", body,
        ])?;
        number(&value).ok_or_else(|| "created pull has no number".to_string())
    }

    pub fn merge(&self, pull: u64, head: &str) -> Result<(), String> {
        self.settle(pull, head, Strategy::Merge)
    }

    pub fn settle(&self, pull: u64, head: &str, strategy: Strategy) -> Result<(), String> {
        let mut last = String::new();
        for turn in 1..=6 {
            let result = self.call(&[
                "pull",
                "merge",
                &pull.to_string(),
                "--head",
                head,
                "--do",
                strategy.wire(),
            ]);
            if result.is_ok() {
                return Ok(());
            }
            last = result.expect_err("failed merge has an error");
            if !retryable(&last) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(turn));
        }
        Err(last)
    }
}

fn retryable(error: &str) -> bool {
    error.contains("405") || error.contains("409")
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
