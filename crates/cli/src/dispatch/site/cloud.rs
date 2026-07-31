use plumb::rig::Site;
use serde_json::Value;
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Response {
    pub status: u16,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bond {
    Yes,
    No,
    Unknown,
}

impl fmt::Display for Bond {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yes => write!(out, "yes"),
            Self::No => write!(out, "no"),
            Self::Unknown => write!(out, "unknown"),
        }
    }
}

pub fn request(url: &str, token: Option<&str>) -> Result<Response, String> {
    let mut child = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--request",
            "GET",
            "--write-out",
            "\n%{http_code}",
            "--config",
            "-",
            "--url",
            url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    let mut config = "header = \"Cache-Control: no-cache\"\n".to_string();
    if let Some(token) = token {
        let safe = token.replace('\\', "\\\\").replace('"', "\\\"");
        config.push_str(&format!("header = \"Authorization: Bearer {safe}\"\n"));
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
    Ok(Response {
        status,
        body: body.to_string(),
    })
}

pub fn binding(site: &Site) -> Bond {
    let url = format!(
        "{}/accounts/{}/workers/domains",
        site.api,
        segment(&site.account)
    );
    let Ok(response) = request(&url, Some(&site.token)) else {
        return Bond::Unknown;
    };
    let Ok(body) = serde_json::from_str::<Value>(&response.body) else {
        return Bond::Unknown;
    };
    if response.status != 200 || body.get("success").and_then(Value::as_bool) != Some(true) {
        return Bond::Unknown;
    }
    let held = body
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|entry| entry.get("hostname").and_then(Value::as_str) == Some(&site.domain));
    if held { Bond::Yes } else { Bond::No }
}

pub fn verify(site: &Site) -> Result<String, String> {
    let user = format!("{}/user/tokens/verify", site.api);
    let first = request(&user, Some(&site.token))?;
    let response = if first.status == 200 {
        first
    } else {
        let account = format!(
            "{}/accounts/{}/tokens/verify",
            site.api,
            segment(&site.account)
        );
        request(&account, Some(&site.token))?
    };
    if response.status != 200 {
        return Err(format!("token verification failed ({})", response.status));
    }
    let body: Value = serde_json::from_str(&response.body)
        .map_err(|error| format!("cannot parse token verification: {error}"))?;
    if body.get("success").and_then(Value::as_bool) == Some(false) {
        return Err("token verification was refused".into());
    }
    let status = body
        .pointer("/result/status")
        .and_then(Value::as_str)
        .ok_or_else(|| "token verification returned no status".to_string())?;
    if status == "active" {
        Ok(status.to_string())
    } else {
        Err(format!("token is {status}"))
    }
}

pub fn worker(site: &Site, name: &str) -> Result<bool, String> {
    let url = format!(
        "{}/accounts/{}/workers/services/{}",
        site.api,
        segment(&site.account),
        segment(name)
    );
    let response = request(&url, Some(&site.token))?;
    match response.status {
        200 => {
            let body: Value = serde_json::from_str(&response.body)
                .map_err(|error| format!("cannot parse worker lookup: {error}"))?;
            Ok(body.get("success").and_then(Value::as_bool) != Some(false))
        }
        404 => Ok(false),
        status => Err(format!("worker lookup failed ({status})")),
    }
}

fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
