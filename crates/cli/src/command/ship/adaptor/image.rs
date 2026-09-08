use super::super::package::context;
use super::container::{fetch, reference};
use crate::shape::release::Spec;
pub(in crate::command) use context::inputs;

const LINUX: &str = "x86_64-unknown-linux-gnu";
const PAYLOAD: &str = "uk.perish.plumb.payload";
const REVISION: &str = "org.opencontainers.image.revision";
use std::process::Command;

pub struct Image<'a> {
    pub(super) spec: &'a Spec,
    pub(super) execution: Option<&'a plumb::config::Execution>,
}

pub fn image(spec: &Spec) -> Image<'_> {
    Image {
        spec,
        execution: None,
    }
}

impl Image<'_> {
    pub(super) fn save(&self, reference: &str) -> Result<std::path::PathBuf, String> {
        let path = self.workload()?;
        let status = self
            .docker()?
            .args(["save", "--output"])
            .arg(&path)
            .arg(reference)
            .current_dir(&self.spec.root)
            .status()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if status.success() && path.is_file() {
            Ok(path)
        } else {
            Err(format!("image attachment left no {}", path.display()))
        }
    }

    pub fn build(
        &self,
        version: &str,
        commit: &str,
        artifacts: &std::path::Path,
    ) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let file = self.spec.root.join("Containerfile");
        if self.spec.binary() && !file.is_file() {
            return Err(format!("declared image has no {}", file.display()));
        }
        let (seat, payload) = if self.spec.binary() {
            self.payload(artifacts, version)?
        } else if !super::container::hex(commit, 40) {
            return Err("PLUMB_RELEASE_COMMIT binds an image that carries no archive".into());
        } else {
            (self.spec.root.clone(), commit.to_string())
        };
        let reference = reference(oci, version);
        let mut command = self.docker()?;
        command.args([
            "build",
            "--platform",
            "linux/amd64",
            "--network",
            "host",
            "--label",
            &format!("{}={payload}", self.mark()),
            "--tag",
            &reference,
            "--file",
        ]);
        if self.spec.binary() {
            command.arg(&file).arg(&seat);
        } else {
            command
                .args(["Containerfile", "-"])
                .stdin(context::archive(self.spec)?);
        }
        let status = command
            .current_dir(&self.spec.root)
            .status()
            .map_err(|error| format!("cannot build image: {error}"))?;
        if !status.success() {
            return Err("image build failed".into());
        }
        Ok(format!("built {reference} carrying {payload}"))
    }

    fn published(&self, archive: &str, version: &str) -> Result<std::path::PathBuf, String> {
        if self.spec.authority.is_empty() {
            return Err(format!(
                "{archive} is absent and this release names no authority"
            ));
        }
        let channel = crate::command::release::channel(version)?;
        let url = format!(
            "{}/v1/releases/{channel}/{version}/seal.json",
            self.spec.authority.trim_end_matches('/')
        );
        let seal = crate::command::release::verify::optional(&url)?
            .ok_or_else(|| format!("the authority serves no seal at {url}"))?;
        let remote = seal
            .get("artifacts")
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flatten()
            .map(|(_, held)| held)
            .find(|held| held.get("name").and_then(serde_json::Value::as_str) == Some(archive))
            .ok_or_else(|| format!("the published seal carries no {archive}"))?;
        let held = |key: &str| {
            remote
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let seat = self.spec.root.join("target/payload");
        std::fs::create_dir_all(&seat)
            .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
        let path = seat.join(archive);
        fetch(&held("url"), &path)?;
        let digest = crate::command::release::record::digest(&path)?.0;
        if digest != held("sha256") {
            return Err(format!(
                "published payload drift: {} serves {digest} while the seal records {}",
                held("url"),
                held("sha256")
            ));
        }
        Ok(path)
    }

    fn mark(&self) -> &'static str {
        if self.spec.binary() {
            PAYLOAD
        } else {
            REVISION
        }
    }

    fn payload(
        &self,
        artifacts: &std::path::Path,
        version: &str,
    ) -> Result<(std::path::PathBuf, String), String> {
        let target = self
            .spec
            .target
            .iter()
            .find(|held| held.triple == LINUX)
            .ok_or_else(|| format!("an image attachment requires the {LINUX} target"))?;
        let archive = target.archive.clone();
        let source = artifacts.join(&archive);
        let source = if source.is_file() {
            source
        } else {
            self.published(&archive, version)?
        };
        let file = std::fs::File::open(&source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        let seat = self.spec.root.join("target/image");
        let _ = std::fs::remove_dir_all(&seat);
        std::fs::create_dir_all(&seat)
            .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
        tar::Archive::new(flate2::read::GzDecoder::new(file))
            .unpack(&seat)
            .map_err(|error| format!("cannot open {}: {error}", source.display()))?;
        for binary in &self.spec.binaries {
            let held = seat.join(binary);
            if !held.is_file() {
                return Err(format!("{} carries no {binary}", source.display()));
            }
        }
        let payload = crate::command::release::record::digest(&source)?.0;
        Ok((seat, payload))
    }

    pub(super) fn docker(&self) -> Result<Command, String> {
        self.execution.map_or_else(
            || Ok(Command::new("docker")),
            |execution| execution.command("docker"),
        )
    }
}
