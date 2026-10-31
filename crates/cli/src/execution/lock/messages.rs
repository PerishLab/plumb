use super::policy::{METHODS, PREFIX};
use serde::Serialize;
use serde_json::Value;
use std::io::BufRead;

#[derive(Serialize)]
pub(crate) struct Finding {
    operation: String,
    package: String,
    target: Value,
    spans: Value,
}

#[derive(Default, Serialize)]
pub(crate) struct Messages {
    pub findings: Vec<Finding>,
    pub artifacts: Vec<Value>,
}

pub(super) fn read(input: impl BufRead) -> Result<Messages, String> {
    let mut held = Messages::default();
    let mut finished = false;
    for line in input.lines() {
        let line = line.map_err(|error| error.to_string())?;
        let message: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid Cargo JSON message: {error}"))?;
        if finished {
            return Err("Cargo emitted messages after build-finished".into());
        }
        match message.get("reason").and_then(Value::as_str) {
            Some("compiler-message") => held.message(&message)?,
            Some("compiler-artifact") => held.artifacts.push(message),
            Some("build-script-executed") => (),
            Some("build-finished")
                if message.get("success").and_then(Value::as_bool) == Some(true) =>
            {
                finished = true
            }
            Some("build-finished") => {
                return Err("Rust compilation failed; no complete semantic report".into());
            }
            _ => return Err("unrecognized Cargo JSON message reason".into()),
        }
    }
    if !finished {
        return Err("Cargo did not report a successful build-finished".into());
    }
    held.findings.sort_by_cached_key(|finding| {
        serde_json::to_string(finding).expect("compiler finding serializes")
    });
    Ok(held)
}

impl Messages {
    fn message(&mut self, message: &Value) -> Result<(), String> {
        let diagnostic = message
            .get("message")
            .ok_or("Cargo compiler-message has no diagnostic")?;
        let selected = diagnostic.pointer("/code/code").and_then(Value::as_str)
            == Some("clippy::disallowed_methods");
        let operation = diagnostic
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|child| child.get("message").and_then(Value::as_str))
            .find_map(|message| message.strip_prefix(PREFIX));
        if !selected || operation.is_none() {
            if let Some(rendered) = diagnostic.get("rendered").and_then(Value::as_str) {
                eprint!("{rendered}");
            }
            return Ok(());
        }
        let operation = operation.expect("selected diagnostic marker");
        if !METHODS.contains(&operation) {
            return Err("compiler returned an unknown managed lock operation".into());
        }
        let reference = format!("`{operation}`");
        if !diagnostic
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains(&reference))
        {
            return Err("managed lock marker differs from the compiler-resolved method".into());
        }
        let package = message
            .get("package_id")
            .and_then(Value::as_str)
            .ok_or("Cargo diagnostic has no package_id")?;
        let target = message
            .get("target")
            .filter(|value| value.is_object())
            .ok_or("Cargo diagnostic has no target")?;
        let spans = diagnostic
            .get("spans")
            .and_then(Value::as_array)
            .filter(|spans| !spans.is_empty())
            .ok_or("Cargo lock diagnostic has no spans")?;
        self.findings.push(Finding {
            operation: operation.into(),
            package: package.into(),
            target: target.clone(),
            spans: Value::Array(spans.clone()),
        });
        Ok(())
    }
}
