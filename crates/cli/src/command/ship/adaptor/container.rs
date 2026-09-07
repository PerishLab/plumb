use super::image::Image;
use crate::command::ship::package::project::{Source, fetch as fetch_workload};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Request<'a> {
    pub version: &'a str,
    pub commit: &'a str,
    pub artifacts: &'a Path,
    pub credential: &'a str,
    pub reuse: &'a str,
}

pub fn run(carrier: &Image<'_>, request: Request<'_>) -> Result<String, String> {
    Exact { carrier }.run(request)
}

struct Exact<'a, 'b> {
    carrier: &'a Image<'b>,
}

impl Exact<'_, '_> {
    fn run(&self, request: Request<'_>) -> Result<String, String> {
        let oci = self
            .carrier
            .spec
            .oci
            .as_ref()
            .ok_or_else(|| format!("{} has no image attachment", self.carrier.spec.product))?;
        let source = Source::parse(request.reuse)?;
        let reference = reference(oci, request.version);
        let workload = match source.kind.as_str() {
            "none" => {
                self.carrier
                    .build(request.version, request.commit, request.artifacts)?;
                self.save(&reference)?
            }
            "workload" => self.load(&reference, &source.source)?,
            "url" => return Err("a held publication URL must skip the image action".into()),
            _ => unreachable!(),
        };
        let provenance = self.carrier.carried(&reference)?;
        let publication = self.carrier.project(request.version, request.credential)?;
        serde_json::to_string(&serde_json::json!({
            "format": "plumb.image-project/v1",
            "version": request.version,
            "workload": workload,
            "publication": publication,
            "provenance": provenance,
        }))
        .map_err(|error| format!("cannot encode image project: {error}"))
    }

    fn save(&self, reference: &str) -> Result<PathBuf, String> {
        let path = self.workload()?;
        let status = self
            .carrier
            .docker()
            .args(["save", "--output"])
            .arg(&path)
            .arg(reference)
            .current_dir(&self.carrier.spec.root)
            .status()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if status.success() && path.is_file() {
            Ok(path)
        } else {
            Err(format!("image attachment left no {}", path.display()))
        }
    }

    fn load(&self, reference: &str, source: &str) -> Result<PathBuf, String> {
        let path = self.workload()?;
        std::fs::write(&path, fetch_workload("image", source)?)
            .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
        let output = self
            .carrier
            .docker()
            .args(["load", "--input"])
            .arg(&path)
            .current_dir(&self.carrier.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if !output.status.success() {
            return Err("cannot load reusable image workload".into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut loaded = stdout.lines().filter_map(|line| {
            line.strip_prefix("Loaded image: ")
                .or_else(|| line.strip_prefix("Loaded image ID: "))
        });
        let image = loaded
            .next()
            .ok_or_else(|| "docker load named no reusable image".to_string())?;
        if loaded.next().is_some() {
            return Err("reusable image workload must carry exactly one image".into());
        }
        self.carrier.carried(image)?;
        self.carrier.command(["tag", image, reference])?;
        Ok(path)
    }

    fn workload(&self) -> Result<PathBuf, String> {
        let seat = self.carrier.spec.root.join("target/image");
        std::fs::create_dir_all(&seat)
            .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
        Ok(seat.join(format!("{}-image.tar", self.carrier.spec.product)))
    }
}

pub(super) fn hex(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl Image<'_> {
    pub(super) fn identity(&self, reference: &str) -> Result<String, String> {
        let held = self.inspect(reference, "{{.Id}}")?;
        if !held
            .strip_prefix("sha256:")
            .is_some_and(|value| hex(value, 64))
        {
            return Err("image declares no valid content identity".into());
        }
        Ok(held)
    }

    pub(super) fn digest(&self, reference: &str) -> Result<String, String> {
        let text = self.inspect(reference, "{{json .RepoDigests}}")?;
        let digests: Vec<String> = serde_json::from_str(&text)
            .map_err(|error| format!("cannot read image repository digests: {error}"))?;
        let repository = reference
            .rsplit_once(':')
            .ok_or("image reference has no tag")?
            .0;
        let prefix = format!("{repository}@sha256:");
        let mut selected = digests.iter().filter_map(|held| held.strip_prefix(&prefix));
        let digest = selected
            .next()
            .ok_or("image has no digest for its publication repository")?;
        if !hex(digest, 64) || selected.next().is_some() {
            return Err("image has no unique valid publication digest".into());
        }
        Ok(format!("sha256:{digest}"))
    }

    fn inspect(&self, reference: &str, format: &str) -> Result<String, String> {
        let output = self
            .docker()
            .args(["image", "inspect", "--format", format, reference])
            .current_dir(&self.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if !output.status.success() {
            return Err(format!("cannot inspect image {reference}"));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

pub(super) fn reference(oci: &crate::shape::release::Oci, version: &str) -> String {
    format!("{}/{}:{version}", oci.registry, oci.image)
}

pub(super) fn fetch(url: &str, path: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--output",
        ])
        .arg(path)
        .arg(url)
        .status()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cannot read the published payload {url}"))
    }
}
