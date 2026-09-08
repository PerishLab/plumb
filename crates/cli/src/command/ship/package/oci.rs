use super::registry::{Session, output, status};
use crate::command::ship::attachment::Identity;
use crate::shape::release::Spec;
use std::path::Path;

pub(in crate::command::ship) struct Image {
    session: Session,
    source: String,
    digest: String,
    pub provenance: String,
}

impl Image {
    pub fn read(spec: &Spec, archive: &Path) -> Result<Self, String> {
        single(archive)?;
        let session = Session::open()?;
        let source = format!(
            "ocidir://{}:workload",
            session.path().join("image").display()
        );
        status(
            session
                .command()
                .args(["image", "import", &source])
                .arg(archive),
        )?;
        let digest = output(session.command().args(["image", "digest", &source]))?;
        let digest = digest.trim().to_string();
        if !digest
            .strip_prefix("sha256:")
            .is_some_and(|value| hex(value, 64))
        {
            return Err("image declares no valid manifest digest".into());
        }
        let source = format!("{}@{digest}", source.trim_end_matches(":workload"));
        let body =
            output(
                session
                    .command()
                    .args(["image", "config", &source, "--format", "{{json .}}"]),
            )?;
        let body: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| format!("cannot read image configuration: {error}"))?;
        let (mark, width) = if spec.binary() {
            ("uk.perish.plumb.payload", 64)
        } else {
            ("org.opencontainers.image.revision", 40)
        };
        let provenance = body["config"]["Labels"][mark].as_str().unwrap_or_default();
        if !hex(provenance, width) {
            return Err(format!("image declares no valid {mark}"));
        }
        Ok(Self {
            session,
            source,
            digest,
            provenance: provenance.to_string(),
        })
    }

    pub fn publish(&self, spec: &Spec, credential: &str) -> Result<String, String> {
        let oci = spec.oci.as_ref().ok_or("image declares no registry")?;
        let identity = Identity {
            user: &oci.account,
            token: crate::command::ship::attachment::credential(credential)?,
        };
        self.session.login(&oci.registry, &identity)?;
        let repository = format!("{}/{}", oci.registry, oci.image);
        let anchor = format!("{repository}:{}", self.digest.replace(':', "-"));
        status(
            self.session
                .command()
                .args(["image", "copy", &self.source, &anchor]),
        )?;
        let held = output(self.session.command().args(["image", "digest", &anchor]))?;
        if held.trim() != self.digest {
            return Err("published image drift at content reference".into());
        }
        let published = format!("{repository}@{}", self.digest);
        let readback = format!(
            "ocidir://{}@{}",
            self.session.path().join("readback").display(),
            self.digest
        );
        status(self.session.anonymous().args([
            "image",
            "copy",
            &published,
            &readback,
            "--force-recursive",
        ]))
        .map_err(|_| "published image or its content cannot be read anonymously".to_string())?;
        let held = output(
            self.session
                .anonymous()
                .args(["image", "digest", &readback]),
        )?;
        if held.trim() != self.digest {
            return Err("anonymous image readback changed content identity".into());
        }
        Ok(format!(
            "https://{}/v2/{}/manifests/{}",
            oci.registry, oci.image, self.digest
        ))
    }
}

fn single(path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot open image workload: {error}"))?;
    let mut archive = tar::Archive::new(file);
    let mut found = false;
    for entry in archive
        .entries()
        .map_err(|error| format!("cannot read image workload: {error}"))?
    {
        let mut entry =
            entry.map_err(|error| format!("cannot read image workload entry: {error}"))?;
        if entry.path().map_err(|error| error.to_string())? != Path::new("manifest.json") {
            continue;
        }
        if found || !entry.header().entry_type().is_file() || entry.size() > 1024 * 1024 {
            return Err("image workload has no unique bounded manifest".into());
        }
        let body: Vec<serde_json::Value> = serde_json::from_reader(&mut entry)
            .map_err(|error| format!("cannot read image workload manifest: {error}"))?;
        if body.len() != 1 {
            return Err("reusable image workload must carry exactly one image".into());
        }
        found = true;
    }
    if !found {
        return Err("image workload carries no Docker archive manifest".into());
    }
    Ok(())
}

fn hex(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
