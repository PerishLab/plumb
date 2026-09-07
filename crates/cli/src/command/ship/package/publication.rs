use crate::command::release::ReleaseMarker;
use crate::command::workflow::record::Binding;
use std::path::Path;

pub(in crate::command::ship) struct Image {
    pub binding: Binding,
    prefix: String,
}

pub(in crate::command::ship) fn image(marker: &ReleaseMarker) -> Result<Image, String> {
    let oci = marker
        .spec()
        .oci
        .as_ref()
        .ok_or("marker declares no OCI resource")?;
    let binding = Binding::new(
        &marker.digest()?,
        &format!("oci://{}/{}", oci.registry, oci.image),
    )?;
    Ok(Image {
        binding,
        prefix: format!(
            "https://{}/v2/{}/manifests/sha256:",
            oci.registry, oci.image
        ),
    })
}

impl Image {
    pub fn resolve(
        &self,
        path: Option<&Path>,
        source: Option<&str>,
    ) -> Result<Option<String>, String> {
        let held = self.binding.resolve(path, source)?;
        if let Some(source) = &held {
            self.verify(source)?;
        }
        Ok(held)
    }

    pub fn verify(&self, source: &str) -> Result<(), String> {
        let digest = source
            .strip_prefix(&self.prefix)
            .ok_or("OCI publication binding names a different resource")?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("OCI publication binding requires a canonical SHA256 digest".into());
        }
        Ok(())
    }
}
