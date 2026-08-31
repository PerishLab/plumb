use plumb::rig::Site;
use serde_json::Value;
use std::collections::BTreeMap;

pub struct Vantage {
    vars: BTreeMap<String, String>,
}

impl Vantage {
    pub fn new(site: &Site) -> Self {
        Self {
            vars: BTreeMap::from([
                ("CLOUDFLARE_ACCOUNT_ID".into(), site.account.clone()),
                ("CLOUDFLARE_API_TOKEN".into(), site.token.clone()),
                ("CLOUDFLARE_API_URL".into(), site.api.clone()),
            ]),
        }
    }

    pub fn get(&self, seat: &str) -> Result<Value, String> {
        self.reach("GET", seat, None)
    }

    fn reach(&self, method: &str, seat: &str, body: Option<&Value>) -> Result<Value, String> {
        let account = self
            .vars
            .get("CLOUDFLARE_ACCOUNT_ID")
            .ok_or_else(|| "no Cloudflare account is declared".to_string())?;
        let token = self
            .vars
            .get("CLOUDFLARE_API_TOKEN")
            .ok_or_else(|| "no Cloudflare token is held".to_string())?;
        let base = self
            .vars
            .get("CLOUDFLARE_API_URL")
            .map(String::as_str)
            .unwrap_or("https://api.cloudflare.com/client/v4");
        let url = format!("{}/accounts/{account}/{seat}", base.trim_end_matches('/'));
        let mut command = std::process::Command::new("curl");
        command
            .args([
                "--silent",
                "--show-error",
                "--location",
                "--request",
                method,
            ])
            .arg("--header")
            .arg(format!("Authorization: Bearer {token}"))
            .arg("--header")
            .arg("Content-Type: application/json");
        if let Some(body) = body {
            command.arg("--data").arg(body.to_string());
        }
        let output = command
            .arg(&url)
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        let held: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot parse {url}: {error}"))?;
        if held.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(format!("cloudflare refused {method} {seat}: {}", held));
        }
        Ok(held.get("result").cloned().unwrap_or(Value::Null))
    }
}
