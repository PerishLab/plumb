use super::Client;
use serde_json::Value;

impl Client {
    pub fn patch(&self, route: &str, body: Value) -> Result<u16, String> {
        if !route.is_empty() {
            return Err("forgejo: repository patch route is not supported".into());
        }
        self.call(&["repo", "edit", "--body", &body.to_string()])?;
        Ok(200)
    }

    pub fn unset(&self, name: &str) -> Result<u16, String> {
        self.call(&["secret", "delete", name])?;
        Ok(204)
    }

    pub fn remove(&self) -> Result<u16, String> {
        self.call(&["repo", "delete"])?;
        Ok(204)
    }
}
