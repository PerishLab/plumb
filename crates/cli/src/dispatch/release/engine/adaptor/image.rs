use super::super::super::model::Spec;

const LINUX: &str = "x86_64-unknown-linux-gnu";
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Image<'a> {
    spec: &'a Spec,
}

pub fn image(spec: &Spec) -> Image<'_> {
    Image { spec }
}

impl Image<'_> {
    pub fn build(&self, version: &str, artifacts: &std::path::Path) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let file = self.spec.root.join("Containerfile");
        if !file.is_file() {
            return Err(format!("declared image has no {}", file.display()));
        }
        let seat = self.payload(artifacts)?;
        let reference = reference(oci, version);
        self.command([
            "build",
            "--network",
            "host",
            "--tag",
            &reference,
            "--file",
            &file.to_string_lossy(),
            &seat.to_string_lossy(),
        ])?;
        Ok(format!("built {reference}"))
    }

    fn payload(&self, artifacts: &std::path::Path) -> Result<std::path::PathBuf, String> {
        let target = self
            .spec
            .target
            .iter()
            .find(|held| held.triple == LINUX)
            .ok_or_else(|| format!("an image attachment requires the {LINUX} target"))?;
        let source = artifacts.join(&target.archive);
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
        Ok(seat)
    }

    pub fn publish(
        &self,
        version: &str,
        identity: &crate::dispatch::ship::Identity<'_>,
    ) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let reference = reference(oci, version);
        self.login(&oci.registry, identity)?;
        self.command(["push", &reference])?;
        let digest = self.digest(&reference)?;
        let published = format!("{}/{}@{digest}", oci.registry, oci.image);
        self.command(["manifest", "inspect", &published])?;
        Ok(format!("published {reference} as {digest}"))
    }

    fn login(
        &self,
        registry: &str,
        identity: &crate::dispatch::ship::Identity<'_>,
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

    fn digest(&self, reference: &str) -> Result<String, String> {
        let output = Command::new("docker")
            .args([
                "image",
                "inspect",
                "--format",
                "{{index .RepoDigests 0}}",
                reference,
            ])
            .current_dir(&self.spec.root)
            .output()
            .map_err(|error| format!("cannot run docker: {error}"))?;
        if !output.status.success() {
            return Err("image attachment kept no published digest".into());
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        text.split_once('@')
            .map(|(_, digest)| digest.to_string())
            .ok_or_else(|| format!("image digest is not addressable: {text}"))
    }

    fn command<const N: usize>(&self, args: [&str; N]) -> Result<(), String> {
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

fn reference(oci: &super::super::super::model::Oci, version: &str) -> String {
    format!("{}/{}:{version}", oci.registry, oci.image)
}
