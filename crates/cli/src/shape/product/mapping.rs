#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mapping {
    provider: String,
    access: String,
    pub account: String,
    pub resource: String,
}

impl Mapping {
    pub fn judge(&self) -> Result<(), String> {
        if self.provider != "cfworker" || self.access != "public" {
            return Err("requires cfworker and explicit public access".into());
        }
        if !super::worker::account(&self.account) || !super::worker::slug(&self.resource) {
            return Err(
                "static mapping needs an explicit account and bounded Worker resource".into(),
            );
        }
        Ok(())
    }
}
