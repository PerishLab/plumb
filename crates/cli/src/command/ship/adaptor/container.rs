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
        let publication = self.carrier.project(request.version, request.credential)?;
        serde_json::to_string(&serde_json::json!({
            "format": "plumb.image-project/v1",
            "version": request.version,
            "workload": workload,
            "publication": publication,
        }))
        .map_err(|error| format!("cannot encode image project: {error}"))
    }

    fn save(&self, reference: &str) -> Result<PathBuf, String> {
        let path = self.workload()?;
        let status = Command::new("docker")
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
        let output = Command::new("docker")
            .args(["load", "--input"])
            .arg(&path)
            .current_dir(&self.carrier.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if !output.status.success() {
            return Err("cannot load reusable image workload".into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let loaded = stdout
            .lines()
            .find_map(|line| {
                line.strip_prefix("Loaded image: ")
                    .or_else(|| line.strip_prefix("Loaded image ID: "))
            })
            .ok_or_else(|| "docker load named no reusable image".to_string())?;
        self.carrier.carried(loaded)?;
        self.carrier.command(["tag", loaded, reference])?;
        Ok(path)
    }

    fn workload(&self) -> Result<PathBuf, String> {
        let seat = self.carrier.spec.root.join("target/image");
        std::fs::create_dir_all(&seat)
            .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
        Ok(seat.join(format!("{}-image.tar", self.carrier.spec.product)))
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
