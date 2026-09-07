use crate::shape::release::Spec;
use serde_json::{Value, json};
use std::path::Path;

pub(super) struct Governance {
    marker: Option<crate::command::release::ReleaseMarker>,
    ambient: Option<Spec>,
}

impl Governance {
    pub fn resolve(root: &Path, marker: &str, exact: bool) -> Result<Self, String> {
        let held = exact
            .then(|| crate::command::release::snapshot(marker))
            .transpose()?;
        let ambient = held.is_none().then(|| Spec::controller(root)).transpose()?;
        Ok(Self {
            marker: held,
            ambient,
        })
    }

    pub fn spec(&self) -> &Spec {
        self.marker
            .as_ref()
            .map(crate::command::release::ReleaseMarker::spec)
            .or(self.ambient.as_ref())
            .expect("governance always carries one release specification")
    }

    pub fn marker(&self) -> Result<&crate::command::release::ReleaseMarker, String> {
        self.marker
            .as_ref()
            .ok_or_else(|| "production requires an exact release marker".into())
    }
}

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
            Err("ship request configuration or product profile differs from its governance".into())
        }
    }
}
