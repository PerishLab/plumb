use plumb::lane::declaration::{Authorities, Declaration};
use plumb::lane::{Action, Address, Capabilities, Name};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    adaptor: Name,
    capabilities: Capabilities,
    authorities: Authorities,
}

impl Binding {
    pub fn judge(&self, target: Address) -> Result<(), String> {
        if target.lane().family() != "preview" || self.adaptor.value() != "static" {
            return Err(
                "first static binding requires preview or preview.<key> and static adaptor".into(),
            );
        }
        let declaration = Declaration::new(
            target,
            self.adaptor.clone(),
            self.capabilities.clone(),
            self.authorities.clone(),
        )?;
        for action in [Action::Inspect, Action::Deploy] {
            declaration.capabilities().require(action)?;
        }
        for action in [Action::Publish, Action::Install, Action::Uninstall] {
            if declaration.capabilities().supports(action) {
                return Err(
                    "static binding cannot declare registry or installation actions".into(),
                );
            }
        }
        Ok(())
    }
}
