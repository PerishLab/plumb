use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Inspect,
    Build,
    Publish,
    Install,
    Uninstall,
    Deploy,
    Dispose,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Capabilities(BTreeSet<Action>);

impl Capabilities {
    pub fn new(actions: Vec<Action>) -> Result<Self, String> {
        let count = actions.len();
        let actions: BTreeSet<_> = actions.into_iter().collect();
        if actions.len() != count {
            return Err("lane capabilities must not repeat an action".into());
        }
        Ok(Self(actions))
    }

    pub fn supports(&self, action: Action) -> bool {
        self.0.contains(&action)
    }

    pub fn require(&self, action: Action) -> Result<(), String> {
        if self.supports(action) {
            Ok(())
        } else {
            Err(format!("lane adaptor does not declare {action:?}"))
        }
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let actions = Vec::<Action>::deserialize(reader)?;
        Self::new(actions).map_err(serde::de::Error::custom)
    }
}
