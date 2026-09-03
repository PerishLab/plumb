use crate::shape::release::Spec;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub(super) struct Binding<'a> {
    configuration: Option<&'a str>,
    profile: Option<&'a str>,
}

impl<'a> Binding<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Self {
            configuration: spec.configuration.as_deref(),
            profile: spec.profile.as_deref(),
        }
    }

    pub fn apply(&self, mut request: Value) -> Value {
        if let Some(configuration) = self.configuration {
            request["configuration"] = json!(configuration);
        }
        if let Some(profile) = self.profile {
            request["profile"] = json!(profile);
        }
        request
    }

    pub fn identity(&self, marker: &str) -> Vec<String> {
        let mut held = vec![format!("marker={marker}")];
        if let Some(configuration) = self.configuration {
            held.push(format!("configuration={configuration}"));
        }
        if let Some(profile) = self.profile {
            held.push(format!("profile={profile}"));
        }
        held
    }

    pub fn verify(&self, configuration: Option<&str>, profile: Option<&str>) -> Result<(), String> {
        if configuration == self.configuration && profile == self.profile {
            Ok(())
        } else {
            Err(
                "ship request configuration or product profile differs from the active depot"
                    .into(),
            )
        }
    }
}
