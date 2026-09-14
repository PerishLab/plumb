use super::{Factory, Grant, Minted, detail, field};
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

    pub fn session<T>(
        &self,
        grant: &Grant,
        action: impl FnOnce(&Minted) -> Result<T, String>,
    ) -> Result<T, String> {
        let minted = self.create(grant)?;
        let result = self.exact(&minted.id, grant).and_then(|()| action(&minted));
        match (result, self.revoke(&minted.id)) {
            (result, Ok(())) => result,
            (Ok(_), Err(error)) => Err(format!(
                "projection completed but temporary token {} could not be revoked: {error}",
                minted.id
            )),
            (Err(error), Err(cleanup)) => Err(format!(
                "{error}; temporary token {} could not be revoked: {cleanup}",
                minted.id
            )),
        }
    }

    fn exact(&self, id: &str, grant: &Grant) -> Result<(), String> {
        let reply = self
            .call(&["token", "account", "show", id], None)
            .map_err(detail)?;
        let held = &reply.value;
        let policies = held["policies"]
            .as_array()
            .ok_or("temporary token policies are unreadable")?;
        if field(held, "id")? != id || held["name"] != grant.name || held["status"] != "active" {
            return Err("temporary token identity did not verify".into());
        }
        if held["expires_on"] != grant.expires || policies.len() != 1 {
            return Err("temporary token lifetime or policy count did not verify".into());
        }
        if policies[0]["effect"] != "allow" || policies[0]["resources"] != grant.resource.policy() {
            return Err("temporary token scope did not verify".into());
        }
        let permissions = policies[0]["permission_groups"]
            .as_array()
            .ok_or("temporary token permissions are unreadable")?;
        if permissions.len() != 1 || permissions[0]["id"] != grant.permission {
            return Err("temporary token permissions did not verify".into());
        }
        Ok(())
    }
}
