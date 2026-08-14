use super::super::super::model::Spec;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Image<'a> {
    spec: &'a Spec,
}

pub fn image(spec: &Spec) -> Image<'_> {
    Image { spec }
}

impl Image<'_> {
    pub fn build(&self, version: &str) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let file = self.spec.root.join("Containerfile");
        if !file.is_file() {
            return Err(format!("declared image has no {}", file.display()));
        }
        let reference = reference(oci, version);
        self.command([
            "build",
            "--network",
            "host",
            "--tag",
            &reference,
            "--file",
            &file.to_string_lossy(),
            ".",
        ])?;
        Ok(format!("built {reference}"))
    }

    pub fn publish(&self, version: &str, token: &str) -> Result<String, String> {
        let Some(oci) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        if token.trim().is_empty() {
            return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
        }
        let reference = reference(oci, version);
        self.login(oci, token)?;
        self.command(["push", &reference])?;
        let digest = self.digest(&reference)?;
        let published = format!("{}/{}@{digest}", oci.registry, oci.image);
        self.command(["manifest", "inspect", &published])?;
        Ok(format!("published {reference} as {digest}"))
    }

    fn login(&self, oci: &super::super::super::model::Oci, token: &str) -> Result<(), String> {
        let owner = oci
            .image
            .split('/')
            .next()
            .filter(|held| !held.is_empty())
            .ok_or_else(|| "image attachment must name an owner".to_string())?;
        let mut child = Command::new("docker")
            .args([
                "login",
                &oci.registry,
                "--username",
                owner,
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
            .write_all(token.as_bytes())
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
