use super::{Object, Rules, Source};
use crate::depot::v3;
use std::path::Path;

pub struct Selection<'a> {
    pub source: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub generation: &'a str,
}

impl Rules {
    pub fn exact(root: &Path, selected: Selection<'_>) -> Result<Self, String> {
        let Selection {
            source,
            channel,
            version,
            generation,
        } = selected;
        let base = v3::local_generation(root, generation)?;
        let path = base.join(v3::LEAF);
        if path.is_file() {
            let body = std::fs::read(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            let manifest = v3::Manifest::parse(&body)?;
            let standing = (
                manifest.product.as_str(),
                manifest.channel.as_str(),
                manifest.version.as_str(),
                manifest.kind,
                manifest.generation()?,
            );
            let wanted = (
                "plumb",
                channel,
                version,
                v3::Kind::Configuration,
                generation.to_string(),
            );
            if standing != wanted {
                return Err(format!(
                    "installed depot generation {generation} names another projection"
                ));
            }
            let objects = objects(&manifest);
            return Ok(Self {
                held: Source::V3 {
                    base,
                    manifest,
                    generation: generation.to_string(),
                    objects,
                },
            });
        }
        #[cfg(feature = "depot")]
        {
            let held = v3::Generation::named(
                v3::Query {
                    source,
                    product: "plumb",
                    channel,
                    version,
                    kind: v3::Kind::Configuration,
                },
                generation,
            )?;
            let objects = objects(&held.manifest);
            Ok(Self {
                held: Source::Remote(held, objects),
            })
        }
        #[cfg(not(feature = "depot"))]
        Err(format!(
            "depot generation {generation} is not installed and remote depot support is disabled"
        ))
    }
}

fn objects(manifest: &v3::Manifest) -> Vec<Object> {
    manifest
        .objects
        .iter()
        .map(|held| Object {
            path: held.path.clone(),
            sha256: held.sha256.clone(),
            size: held.size,
        })
        .collect()
}
