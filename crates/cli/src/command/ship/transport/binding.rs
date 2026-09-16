pub(super) use super::super::native::workload::{Workload, materialize};
use crate::shape::release::Spec;
use serde_json::{Value, json};

pub(super) struct Governance {
    marker: crate::command::release::ReleaseMarker,
}

impl Governance {
    pub fn resolve(marker: &str) -> Result<Self, String> {
        let marker = crate::command::release::snapshot(marker)?;
        marker.spec().ship()?;
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
            ("version", &release.version, &self.marker.version),
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
        if request["operation"]["type"] == "package" {
            let action = request["action"]
                .as_str()
                .and_then(|value| value.strip_prefix("ship/produce."))
                .ok_or("package production has no declared Ship action")?;
            if !matches!(
                request["operation"]["operation"]["type"].as_str(),
                Some("cargo" | "npm" | "chart" | "cfworker")
            ) {
                return Err("package production requires one supported medium".into());
            }
            return self.request(&json!({"action": format!("ship/{action}"),
                "operation": request["operation"]["operation"]}));
        }
        let mut operation = request["operation"].clone();
        operation
            .as_object_mut()
            .ok_or("ship request carries no operation")?
            .remove("workloads");
        operation
            .as_object_mut()
            .ok_or("ship request carries no operation")?
            .remove("build");
        let spec = self.spec();
        let expected = match operation["type"].as_str() {
            Some("complete") => json!({"action": "ship/complete"}),
            Some("produce" | "bind" | "publication") if spec.binary() => binary(spec, &operation)?,
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
        if request["action"] != expected["action"] {
            return Err("ship request action differs from its marker contract".into());
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

    pub fn verify(&self, configuration: Option<&str>, profile: Option<&str>) -> Result<(), String> {
        if configuration == self.configuration && profile == self.profile {
            Ok(())
        } else {
            Err("ship request configuration or product profile differs from its governance".into())
        }
    }
}

fn binary(spec: &Spec, operation: &Value) -> Result<Value, String> {
    let action = if matches!(operation["type"].as_str(), Some("produce" | "bind")) {
        let triple = operation["target"]
            .as_str()
            .ok_or("binary request has no target")?;
        if operation["archive"] != spec.target(triple)?.archive {
            return Err("binary request archive differs from its target".into());
        }
        if operation["type"] == "produce" {
            format!("ship/produce.{triple}")
        } else {
            format!("ship/binary.{triple}")
        }
    } else {
        "ship/binary".into()
    };
    Ok(
        json!({"action":action,"roots":super::support::sources(spec)?,
        "projections":[super::support::projection()],"operation":operation}),
    )
}
