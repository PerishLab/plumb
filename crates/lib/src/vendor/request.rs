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
        Err(format!(
            "fetching stable pointer failed ({}): {}",
            response.status, response.value
        ))
    }
}

pub fn send(
    url: &str,
    method: &str,
    body: Option<Value>,
    auth: Option<&str>,
) -> Result<Response, String> {
    let mut command = Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--max-time",
        "120",
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
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    let mut config =
        "header = \"Accept: application/json\"\nheader = \"Cache-Control: no-cache\"\n".to_string();
    if let Some(auth) = auth {
        let safe = auth.replace('\\', "\\\\").replace('"', "\\\"");
        config.push_str(&format!("header = \"Authorization: {safe}\"\n"));
    }
    child
        .stdin
        .take()
        .ok_or_else(|| "cannot open curl input".to_string())?
        .write_all(config.as_bytes())
        .map_err(|error| format!("cannot write curl input: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for curl: {error}"))?;
    if !output.status.success() {
        return Err(said(&output, url));
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

fn said(output: &std::process::Output, url: &str) -> String {
    let held = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if held.is_empty() {
        let code = output
            .status
            .code()
            .map_or_else(|| "a signal".to_string(), |held| format!("exit {held}"));
        return format!("curl reached {url} with {code} and said nothing");
    }
    format!("curl failed for {url}: {held}")
}
