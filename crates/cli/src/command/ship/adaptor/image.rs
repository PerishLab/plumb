use super::super::package::context;
use super::container::{fetch, reference};
use crate::shape::release::Spec;
pub(in crate::command) use context::inputs;

const LINUX: &str = "x86_64-unknown-linux-gnu";
const PAYLOAD: &str = "uk.perish.plumb.payload";
const REVISION: &str = "org.opencontainers.image.revision";
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Image<'a> {
    pub(super) spec: &'a Spec,
}

pub fn image(spec: &Spec) -> Image<'_> {
    Image { spec }
}

impl Image<'_> {
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
        let mut command = Command::new("docker");
        command.args([
            "build",
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

    pub fn publish(&self, version: &str, credential: &str) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let reference = reference(oci, version);
        let publication = self.project(version, credential)?;
        Ok(format!("published {reference} as {publication}"))
    }

    pub(super) fn project(&self, version: &str, credential: &str) -> Result<String, String> {
        let oci = self
            .spec
            .oci
            .as_ref()
            .ok_or_else(|| format!("{} has no image attachment", self.spec.product))?;
        let identity = crate::command::ship::attachment::Identity {
            user: &oci.account,
            token: crate::command::ship::attachment::credential(credential)?,
        };
        let reference = reference(oci, version);
        self.login(&oci.registry, &identity)?;
        self.carried(&reference)?;
        let built = self.identity(&reference)?;
        if self.fetched(&reference)? {
            let held = self.identity(&reference)?;
            if held != built {
                return Err(format!(
                    "published image drift: {reference} carries {held} while this projection carries {built}"
                ));
            }
        } else {
            self.command(["push", &reference])?;
        }
        let digest = self.digest(&reference)?;
        let published = format!("{}/{}@{digest}", oci.registry, oci.image);
        self.command(["manifest", "inspect", &published])?;
        Ok(format!(
            "https://{}/v2/{}/manifests/{digest}",
            oci.registry.trim_end_matches('/'),
            oci.image
        ))
    }

    fn login(
        &self,
        registry: &str,
        identity: &crate::command::ship::attachment::Identity<'_>,
    ) -> Result<(), String> {
        let mut child = Command::new("docker")
            .args([
                "login",
                registry,
                "--username",
                identity.user,
                "--password-stdin",
            ])
            .current_dir(&self.spec.root)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "docker login refused its stdin".to_string())?
            .write_all(identity.token.as_bytes())
            .map_err(|error| format!("cannot send registry token: {error}"))?;
        let status = child
            .wait()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("image attachment login failed".into())
        }
    }

    fn fetched(&self, reference: &str) -> Result<bool, String> {
        let output = Command::new("docker")
            .args(["pull", reference])
            .current_dir(&self.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        Ok(output.status.success())
    }

    pub(super) fn carried(&self, reference: &str) -> Result<String, String> {
        let output = Command::new("docker")
            .args([
                "image",
                "inspect",
                "--format",
                &format!("{{{{index .Config.Labels \"{}\"}}}}", self.mark()),
                reference,
            ])
            .current_dir(&self.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if !output.status.success() {
            return Err(format!("no {reference} to read a payload from"));
        }
        let held = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let width = if self.spec.binary() { 64 } else { 40 };
        if !super::container::hex(&held, width) {
            return Err(format!("{reference} declares no valid {}", self.mark()));
        }
        Ok(held)
    }

    pub(super) fn command<const N: usize>(&self, args: [&str; N]) -> Result<(), String> {
        let status = Command::new("docker")
            .args(args)
            .current_dir(&self.spec.root)
            .status()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("image attachment command failed".into())
        }
    }
}
