mod validation;

use super::declaration::Declaration;
use super::operation::{Intent, Request, State};
use super::{Action, Address, Digest};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Apply,
    Inspect,
    Dispose,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Dispatch(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    schema: String,
    target: Address,
    operation: Operation,
    request: Digest,
    revision: u64,
    expected: State,
    #[serde(deserialize_with = "nullable")]
    source: Option<crate::preview::Source>,
    registration: Digest,
    caller: String,
    workflow: String,
}

impl Dispatch {
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 65536 {
            return Err("lane dispatch exceeds its document budget".into());
        }
        serde_json::from_slice(bytes).map_err(|_| "invalid static lane dispatch document".into())
    }

    pub fn target(&self) -> &Address {
        &self.0.target
    }
    pub fn operation(&self) -> Operation {
        self.0.operation
    }
    pub fn request(&self) -> &Digest {
        &self.0.request
    }
    pub fn revision(&self) -> u64 {
        self.0.revision
    }
    pub fn expected(&self) -> &State {
        &self.0.expected
    }
    pub fn source(&self) -> Option<&crate::preview::Source> {
        self.0.source.as_ref()
    }
    pub fn caller(&self) -> &str {
        &self.0.caller
    }
    pub fn workflow(&self) -> &str {
        &self.0.workflow
    }

    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "cannot encode lane dispatch".into())
    }

    pub fn capability(&self) -> Action {
        match self.operation() {
            Operation::Apply => Action::Deploy,
            Operation::Inspect => Action::Inspect,
            Operation::Dispose => Action::Dispose,
        }
    }

    pub fn qualify(&self, declaration: &Declaration) -> Result<(), String> {
        if declaration.target() != self.target() {
            return Err("dispatch requires the exact declared target".into());
        }
        let wharf = super::Atom::read("wharf")?;
        let authorities = super::declaration::Authorities::new(wharf.clone(), wharf)?;
        if declaration.adaptor().value() != "static" || declaration.authorities() != &authorities {
            return Err("dispatch requires the static Wharf authority binding".into());
        }
        declaration.capabilities().require(self.capability())
    }

    pub fn matches(&self, request: &Request) -> Result<(), String> {
        let context = super::evidence::Context::new(
            self.target().clone(),
            self.request().clone(),
            self.revision(),
            super::Atom::read("static")?,
        )?;
        let expected = if self.operation() == Operation::Inspect {
            None
        } else {
            Some(self.expected())
        };
        if request.context() != &context || request.expected() != expected {
            return Err(
                "lane result differs from the exact dispatch context or precondition".into(),
            );
        }
        let valid = match (self.operation(), request.intent()) {
            (Operation::Inspect, Intent::Inspect {}) | (Operation::Dispose, Intent::Dispose {}) => {
                true
            }
            (Operation::Apply, Intent::Deploy { artifact }) => {
                let source = self.source().ok_or("apply requires source")?;
                let expected = super::Source::new(
                    self.target().repository().into(),
                    source.commit.clone(),
                    source.tree.clone(),
                )?;
                artifact.source() == &expected && artifact.build().is_some()
            }
            _ => false,
        };
        if !valid {
            return Err("lane result differs from the exact static intent/source".into());
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Dispatch {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let parts = Parts::deserialize(reader)?;
        validation::check(&parts).map_err(serde::de::Error::custom)?;
        Ok(Self(parts))
    }
}

#[cfg(test)]
mod tests;

fn nullable<'de, D: Deserializer<'de>>(
    reader: D,
) -> Result<Option<crate::preview::Source>, D::Error> {
    Option::deserialize(reader)
}
