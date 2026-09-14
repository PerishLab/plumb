pub(super) use super::super::native::workload::{Workload, materialize};
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
        operation
            .as_object_mut()
            .ok_or("ship request carries no operation")?
            .remove("build");
        let spec = self.spec();
        let expected = match operation["type"].as_str() {
            Some("bind" | "publication") if spec.binary() => binary(spec, &operation)?,
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
        let stage = if operation["type"] == "bind" {
            "identity/v1"
        } else {
            ""
        };
        let node = self.planned(&expected, stage)?;
        if stage == "identity/v1" {
            let build = &request["operation"]["build"];
            let content = self.planned(&expected, "content/v1")?;
            if build["keys"] != content["keys"] || build["production"] != content["production"] {
                return Err("ship build differs from its marker content plan".into());
            }
        }
        if request["keys"] != node["keys"] {
            return Err("ship request keys differ from its marker plan".into());
        }
        if request["production"] != node["production"] {
            return Err("ship request production differs from its marker plan".into());
        }
        Ok(())
    }

    fn planned(&self, request: &Value, stage: &str) -> Result<Value, String> {
        use super::support::{Contract, Plan, contract, embedded, strings, text};
        let action = text(request, "action")?;
        let binary = action.starts_with("ship/binary");
        let workload = request["operation"]["type"] == "bind";
        let target = if workload {
            Some(self.spec().target(text(&request["operation"], "target")?)?)
        } else {
            None
        };
        let runner = target.map_or("docker", |held| held.runner.as_str());
        let contract = if action == "ship/oci" && self.spec().binary() {
            Contract::Exact
        } else {
            contract(action)
        };
        let release = if workload {
            Some(if stage == "content/v1" {
                self.marker.base()
            } else {
                self.marker.marker.as_str()
            })
        } else {
            (binary || contract != Contract::Portable).then_some(self.marker.version.as_str())
        };
        let target = if workload {
            Some(text(&request["operation"], "target")?)
        } else {
            (binary || contract == Contract::Exact).then_some(self.marker.commit.as_str())
        };
        super::resolve::planned(
            &super::resolve::World {
                evidence: false,
                marker: &self.marker,
                binding: Binding::new(self.spec()),
                inventory: None,
                source: None,
                root: &self.spec().root,
            },
            Plan {
                stage,
                action,
                projections: &strings(request, "projections")?,
                roots: &strings(request, "roots")?,
                runner,
                workload: if workload {
                    release
                } else if action == "ship/oci" && self.spec().binary() {
                    Some(self.marker.marker.as_str())
                } else {
                    (binary || embedded(action)).then_some(self.marker.base())
                },
                release,
                target,
            },
        )
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
    let action = if operation["type"] == "bind" {
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
