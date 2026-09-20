use super::Factory;
use plumb::{config::Cascade as _, rig::Mint};

impl Factory {
    pub fn resolve() -> Result<Self, String> {
        let mut held =
            Mint::default().merge(Mint::env("PLUMB_AUTHORITY").map_err(|e| e.to_string())?);
        held.load()?;
        if held.account.trim().is_empty() || held.token.trim().is_empty() {
            return Err("missing PLUMB_AUTHORITY_ACCOUNT or one of PLUMB_AUTHORITY_TOKEN / PLUMB_AUTHORITY_TOKEN_FILE".into());
        }
        if held.token.contains(['\r', '\n']) {
            return Err("PLUMB_AUTHORITY_TOKEN contains a line break".into());
        }
        Ok(Self::new(
            held.account,
            held.api.trim_end_matches('/').to_string(),
            held.token,
        ))
    }
}
