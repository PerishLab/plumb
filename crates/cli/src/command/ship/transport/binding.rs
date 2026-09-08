use crate::command::release::ReleaseMarker;
use crate::shape::release::Spec;
use plumb::rule::Receipt;
use serde_json::{Value, json};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Workload {
    target: String,
    archive: String,
    url: String,
    receipt: Receipt,
}

fn validate(workloads: &[Workload], marker: &ReleaseMarker, image: bool) -> Result<(), String> {
    let mut held = std::collections::BTreeSet::new();
    for workload in workloads {
        if !held.insert(workload.target.as_str()) {
            return Err("duplicate binary workload target".into());
        }
        if workload.archive != marker.spec().target(&workload.target)?.archive {
            return Err("reused workload archive differs from its target".into());
        }
        if !workload.url.starts_with("https://") {
            return Err("binary workload must use an HTTPS URL".into());
        }
        super::production::contract(marker, &workload.target)?.verify(&workload.receipt)?;
    }
    let required = marker
        .spec()
        .target
        .iter()
        .filter(|target| !image || target.triple == "x86_64-unknown-linux-gnu");
    for target in required {
        if !held.contains(target.triple.as_str()) {
            return Err(format!(
                "ship request has no proven binary workload for {}",
                target.triple
            ));
        }
    }
    if image && marker.spec().binary() && !held.contains("x86_64-unknown-linux-gnu") {
        return Err("image request has no proven Linux binary workload".into());
    }
    Ok(())
}

pub(super) fn materialize(
    root: &std::path::Path,
    workloads: &[Workload],
    marker: &ReleaseMarker,
    image: bool,
) -> Result<(), String> {
    validate(workloads, marker, image)?;
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for workload in workloads {
        let target = root.join(&workload.archive);
        let status = std::process::Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--retry",
                "3",
                "--output",
            ])
            .arg(&target)
            .arg(&workload.url)
            .status()
            .map_err(|error| format!("cannot fetch {}: {error}", workload.url))?;
        if !status.success() {
            return Err(format!(
                "cannot fetch binary workload for {}",
                workload.target
            ));
        }
        workload.receipt.verify(&target)?;
    }
    Ok(())
}

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
        let node = self.planned(&expected)?;
        if request["keys"] != node["keys"] {
            return Err("ship request keys differ from its marker plan".into());
        }
        if request["production"] != node["production"] {
            return Err("ship request production differs from its marker plan".into());
        }
        Ok(())
    }

    fn planned(&self, request: &Value) -> Result<Value, String> {
        use super::support::{Contract, Plan, contract, embedded, strings, text};
        let action = text(request, "action")?;
        let binary = action.starts_with("ship/binary");
        let workload = request["operation"]["type"] == "workload";
        let target = if workload {
            Some(self.spec().target(text(&request["operation"], "target")?)?)
        } else {
            None
        };
        let runner = target.map_or("docker", |held| held.runner.as_str());
        let contract = contract(action);
        let release = if workload {
            Some(self.marker.base())
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
                marker: &self.marker,
                binding: Binding::new(self.spec()),
                inventory: None,
                source: None,
                root: &self.spec().root,
            },
            Plan {
                action,
                projections: &strings(request, "projections")?,
                roots: &strings(request, "roots")?,
                runner,
                workload: (binary || embedded(action)).then_some(self.marker.base()),
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
