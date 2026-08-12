use super::{Client, segment};
use serde_json::Value;

impl Client {
    pub fn patch(&self, route: &str, body: Value) -> Result<u16, String> {
        Ok(self.request("PATCH", route, Some(body))?.status)
    }

    pub fn unset(&self, name: &str) -> Result<u16, String> {
        let route = format!("/actions/secrets/{}", segment(name));
        Ok(self.request("DELETE", &route, None)?.status)
    }

    pub fn remove(&self) -> Result<u16, String> {
        Ok(self.request("DELETE", "", None)?.status)
    }
}
