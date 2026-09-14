use super::Mint;
use std::path::{Path, PathBuf};

impl Default for Mint {
    fn default() -> Self {
        Self {
            account: String::new(),
            api: "https://api.cloudflare.com/client/v4".to_string(),
            token: String::new(),
            file: PathBuf::new(),
        }
    }
}

impl Mint {
    pub fn load(&mut self) -> Result<(), String> {
        read(&mut self.token, &self.file)
    }
}

pub(super) fn read(secret: &mut String, file: &Path) -> Result<(), String> {
    if !secret.is_empty() || file.as_os_str().is_empty() {
        return Ok(());
    }
    let held = std::fs::read_to_string(file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let held = held.trim();
    if held.is_empty() {
        return Err(format!("{} holds no secret", file.display()));
    }
    *secret = held.to_string();
    Ok(())
}
