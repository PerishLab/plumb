use super::model::Artifact;
use serde_json::Value;

pub(super) fn read(stream: &str) -> Result<Vec<Artifact>, String> {
    let mut artifacts = Vec::new();
    let mut finished = false;
    for line in stream.lines().filter(|line| !line.trim().is_empty()) {
        if finished {
            return Err("Cargo emitted a message after build-finished".into());
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid Cargo test message: {error}"))?;
        match value.get("reason").and_then(Value::as_str) {
            Some("compiler-artifact") => artifacts.push(
                serde_json::from_value(value)
                    .map_err(|error| format!("invalid Cargo test artifact: {error}"))?,
            ),
            Some("compiler-message" | "build-script-executed") => (),
            Some("build-finished") if value.get("success") == Some(&Value::Bool(true)) => {
                finished = true;
            }
            Some("build-finished") => return Err("Cargo test compilation did not succeed".into()),
            _ => return Err("unrecognized Cargo test message".into()),
        }
    }
    if !finished {
        return Err("Cargo test compilation has no successful build-finished".into());
    }
    Ok(artifacts)
}
