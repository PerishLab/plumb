use crate::shape::release::Spec;
use serde_json::{Value, json};

pub(super) struct Governance {
    marker: crate::command::release::ReleaseMarker,
}

impl Governance {
    pub fn resolve(marker: &str) -> Result<Self, String> {
        let marker = crate::command::release::snapshot(marker)?;
        super::promotion::verify(&marker)?;
        Ok(Self { marker })
    }

    pub fn spec(&self) -> &Spec {
        self.marker.spec()
    }

    pub fn marker(&self) -> &crate::command::release::ReleaseMarker {
        &self.marker
    }

    pub fn apply(&self, release: &mut plumb::rig::Release) -> Result<(), String> {
        let root = release
            .root
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let expected = self
            .spec()
            .root
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if root != expected {
            return Err("ship execution root differs from the marker repository".into());
        }
        self.checkout()?;
        for (name, actual, expected) in [
            ("channel", &release.channel, &self.marker.channel),
            ("commit", &release.commit, &self.marker.commit),
        ] {
            if !actual.is_empty() && actual != expected {
                return Err(format!("ship execution {name} differs from its marker"));
            }
        }
        release.channel = self.marker.channel.clone();
        release.commit = self.marker.commit.clone();
        release.version = self.marker.version.clone();
        Ok(())
    }

    fn checkout(&self) -> Result<(), String> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.spec().root)
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|error| format!("cannot inspect ship checkout: {error}"))?;
        if !output.status.success()
            || String::from_utf8_lossy(&output.stdout).trim() != self.marker.commit
        {
            return Err("ship checkout differs from its marker commit".into());
        }
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.spec().root)
            .args(["diff-index", "--quiet", &self.marker.commit, "--"])
            .status()
            .map_err(|error| format!("cannot inspect ship tree: {error}"))?;
        if !status.success() {
            return Err("ship tracked tree differs from its marker".into());
        }
        Ok(())
    }

    pub fn request(&self, request: &Value) -> Result<(), String> {
        let mut operation = request["operation"].clone();
        operation
            .as_object_mut()
            .ok_or("ship request carries no operation")?
            .remove("workloads");
        let spec = self.spec();
        let expected = match operation["type"].as_str() {
            Some("workload" | "publication") if spec.binary() => binary(spec, &operation)?,
            _ => {
                let surface: Value =
                    serde_json::from_str(&crate::command::release::plan::surface(spec)?)
                        .map_err(|error| error.to_string())?;
                surface["publication"]["include"]
                    .as_array()
                    .ok_or("ship surface has no publication rows")?
                    .iter()
                    .find(|row| row["operation"] == operation)
                    .cloned()
                    .ok_or("operation is not a declared marker-bound Ship request")?
            }
        };
        for field in ["action", "roots", "projections"] {
            if request[field] != expected[field] {
                return Err(format!("ship request {field} differs from its marker plan"));
            }
        }
        Ok(())
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

fn binary(spec: &Spec, operation: &Value) -> Result<Value, String> {
    let action = if operation["type"] == "workload" {
        let triple = operation["target"]
            .as_str()
            .ok_or("binary request has no target")?;
        if operation["archive"] != spec.target(triple)?.archive {
            return Err("binary request archive differs from its target".into());
        }
        format!("ship/binary.{triple}")
    } else {
        "ship/binary".into()
    };
    Ok(
        json!({"action":action,"roots":super::support::sources(spec)?,
        "projections":[super::support::projection()],"operation":operation}),
    )
}
