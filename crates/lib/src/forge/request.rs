use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Response {
    pub status: u16,
    pub value: Value,
}

pub fn public(url: &str) -> Result<Value, String> {
    let response = send(url, "GET", None, None)?;
    if response.status == 200 {
        Ok(response.value)
    } else {
        Err(failure(
            "fetching stable pointer",
            response.status,
            &response.value,
        ))
    }
}

pub fn send(
    url: &str,
    method: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> Result<Response, String> {
    let mut command = Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--request",
        method,
        "--write-out",
        "\n%{http_code}",
        "--config",
        "-",
    ]);
    if let Some(body) = body {
        command
            .args([
                "--header",
                "Content-Type: application/json",
                "--data-binary",
            ])
            .arg(body.to_string());
    }
    let mut child = command
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if let Some(token) = token {
        let safe = token.replace('\\', "\\\\").replace('"', "\\\"");
        let config = format!(
            "header = \"Authorization: token {safe}\"\nheader = \"Accept: application/json\"\n"
        );
        child
            .stdin
            .take()
            .ok_or_else(|| "cannot open curl input".to_string())?
            .write_all(config.as_bytes())
            .map_err(|error| format!("cannot write curl input: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = text
        .rsplit_once('\n')
        .ok_or_else(|| "curl returned no HTTP status".to_string())?;
    let status = status
        .trim()
        .parse()
        .map_err(|_| format!("invalid HTTP status: {status}"))?;
    let value = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body).unwrap_or_else(|_| Value::String(body.to_string()))
    };
    Ok(Response { status, value })
}

pub fn failure(action: &str, status: u16, value: &Value) -> String {
    let detail = value
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string());
    format!("forgejo: {action} failed ({status}): {detail}")
}
