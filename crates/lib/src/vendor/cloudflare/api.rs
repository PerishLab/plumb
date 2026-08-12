use crate::vendor::send;
use serde_json::Value;

pub struct Fault {
    pub status: u16,
    pub detail: String,
}

pub struct Account {
    pub id: String,
    base: String,
    token: String,
}

impl Account {
    pub fn new(id: String, base: String, token: String) -> Self {
        Self { id, base, token }
    }

    pub fn swap(&self, token: &str) -> Self {
        Self {
            id: self.id.clone(),
            base: self.base.clone(),
            token: token.to_string(),
        }
    }

    pub fn read(&self, route: &str) -> Result<Value, Fault> {
        self.result("GET", route, None)
    }

    pub fn erase(&self, route: &str) -> Result<Value, Fault> {
        self.result("DELETE", route, None)
    }

    pub fn result(&self, method: &str, route: &str, body: Option<Value>) -> Result<Value, Fault> {
        let url = format!("{}{route}", self.base);
        let response = send(&url, method, body, Some(&self.bearing()))
            .map_err(|detail| Fault { status: 0, detail })?;
        let held = &response.value;
        if response.status < 300 && held.get("success").and_then(Value::as_bool) == Some(true) {
            return Ok(held.get("result").cloned().unwrap_or(Value::Null));
        }
        Err(Fault {
            status: response.status,
            detail: complaint(held),
        })
    }

    fn bearing(&self) -> String {
        format!("Bearer {}", self.token)
    }

    pub fn scope(&self) -> String {
        format!("/accounts/{}", self.id)
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "cloudflare refused ({}): {}", self.status, self.detail)
    }
}

impl From<Fault> for String {
    fn from(fault: Fault) -> Self {
        fault.to_string()
    }
}

fn complaint(held: &Value) -> String {
    let listed = held
        .get("errors")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.get("message").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    if listed.is_empty() {
        held.to_string()
    } else {
        listed
    }
}
