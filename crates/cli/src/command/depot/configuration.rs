use super::store;
use crate::shape::depot as record;
use plumb::rig::Rig;
use std::path::Path;
use std::process::Output;

pub struct Tree<'a>(pub &'a Path);

impl Tree<'_> {
    pub fn publish(&self, raw: &str, from: &str, dry: bool) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let marker = crate::command::release::ReleaseMarker::bound(self.0, raw)?;
        let spec = marker.spec();
        let proof = marker.digest()?;
        if marker.commit != self.commit()? {
            return Err(format!(
                "release marker {} does not stand at HEAD",
                marker.marker
            ));
        }
        if from.is_empty() {
            return Err("configuration publication requires an explicit --from directory".into());
        }
        let depot = spec.derivative(plumb::depot::v3::Kind::Configuration)?;
        let product = crate::command::release::Product::new(spec);
        let release = product.depot();
        let binding = release.validator(&marker.marker, true)?;
        let bundle = plumb::depot::v3::Bundle::read(
            Path::new(from),
            plumb::depot::v3::Identity {
                product: marker.product.clone(),
                channel: marker.channel.clone(),
                version: marker.marker.clone(),
                marker: plumb::depot::v3::Marker {
                    name: marker.marker.clone(),
                    sha256: proof.clone(),
                },
                kind: plumb::depot::v3::Kind::Configuration,
            },
        )?;
        let plan = record::Batch::compatibility(
            &bundle,
            record::Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: super::super::clock::mark()?,
                commit: marker.commit.clone(),
            },
        )?;
        crate::command::release::validate_depot(spec, &binding, &plan, None)?;
        let held = (|| {
            if dry {
                String::from_utf8(bundle.manifest.encode()?)
                    .map_err(|error| format!("depot manifest is not UTF-8: {error}"))
            } else {
                rig.depot.authority.load()?;
                store::Remote::new(&rig.depot.authority)?.publish(
                    &bundle,
                    &depot.source,
                    super::super::clock::ahead(0)?,
                )
            }
        })();
        let after = crate::command::release::ReleaseMarker::bound(self.0, &marker.marker)?;
        if after.digest()? != proof {
            return Err(format!(
                "release marker {} drifted while depot was deriving configuration",
                marker.marker
            ));
        }
        held
    }

    pub fn commit(&self) -> Result<String, String> {
        let output = self.git(&["rev-parse", "HEAD"])?;
        if !output.status.success() {
            return Err("cannot read HEAD".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn git(&self, args: &[&str]) -> Result<Output, String> {
        std::process::Command::new("git")
            .current_dir(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}
