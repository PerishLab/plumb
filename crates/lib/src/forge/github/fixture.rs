use super::Pull;
use crate::delivery::{Patience, Settle, Squash};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

pub(super) struct Fixture {
    pub(super) seat: String,
    pub(super) command: PathBuf,
    pub(super) patience: Patience,
}

impl Fixture {
    pub fn settle(&self, number: u64, squash: &Squash) -> Result<(), String> {
        let gh = |args: &[String]| self.run(args);
        let settle = Settle {
            repository: &self.seat,
            noun: "land",
            patience: self.patience,
            gh: &gh,
        };
        settle.wait(&squash.candidate)?;
        settle.merge(number, squash)
    }

    fn run(&self, args: &[String]) -> Result<Vec<u8>, String> {
        let output = Command::new(&self.command)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run gh: {error}"))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let named = args
            .iter()
            .take_while(|arg| !arg.starts_with('-'))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        Err(format!(
            "gh {named} on {} failed: {}",
            self.seat,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub(super) fn pull(value: &Value) -> Option<Pull> {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Some(Pull {
        node: text("id"),
        number: value.get("number")?.as_u64()?,
        url: text("url"),
        head: text("headRefOid"),
        title: text("title"),
        body: text("body"),
    })
}
