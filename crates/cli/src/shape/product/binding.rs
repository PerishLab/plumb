use plumb::lane::declaration::{Authorities, Declaration};
use plumb::lane::{Action, Address, Atom, Capabilities};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    adaptor: Atom,
    capabilities: Capabilities,
    authorities: Authorities,
}

impl Binding {
    pub fn declaration(&self, target: Address) -> Result<Declaration, String> {
        Declaration::new(
            target,
            self.adaptor.clone(),
            self.capabilities.clone(),
            self.authorities.clone(),
        )
    }

    pub fn judge(&self, target: Address) -> Result<(), String> {
        if self.adaptor.value() != "static" {
            return Err("static binding requires the static adaptor".into());
        }
        let declaration = self.declaration(target)?;
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
