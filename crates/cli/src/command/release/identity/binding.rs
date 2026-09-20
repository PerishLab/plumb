use crate::shape::release::Spec;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Annotation {
    schema: String,
    product: String,
    marker: String,
    configuration: Configuration,
    profile: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Configuration {
    channel: String,
    version: String,
    generation: String,
}

pub(super) struct Identity {
    pub product: String,
    pub authority: String,
    pub configuration: Option<String>,
    pub profile: Option<String>,
    pub spec: Box<Spec>,
}

pub(super) fn resolve(
    message: &str,
    root: &Path,
    marker: &str,
) -> Result<Option<Identity>, String> {
    if !message.starts_with('{') {
        return Ok(None);
    }
    let held: Annotation = serde_json::from_str(message)
        .map_err(|error| format!("cannot parse release marker annotation: {error}"))?;
    held.resolve(root, marker).map(Some)
}

impl Annotation {
    fn resolve(self, root: &Path, marker: &str) -> Result<Identity, String> {
        if self.schema != "plumb.release-marker/v2" {
            return Err(format!(
                "unknown release marker annotation schema {}",
                self.schema
            ));
        }
        if self.marker != marker {
            return Err(format!(
                "release marker {marker} annotation names {}",
                self.marker
            ));
        }
        let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
        let seat = plumb::depot::Rules::exact(
            &plumb::depot::root(Path::new(""))?,
            plumb::depot::Selection {
                source: &rig.rules.source,
                channel: &self.configuration.channel,
                version: &self.configuration.version,
                generation: &self.configuration.generation,
            },
        )?;
        let target = crate::shape::product::at(root, &rig.rules.source, &seat)?;
        let profile = target
            .profile
            .as_ref()
            .ok_or_else(|| "release marker configuration carries no Product Profile".to_string())?;
        let configuration = profile.configuration.clone();
        let digest = profile.digest.clone();
        let standing = (
            target.product.as_str(),
            profile.configuration.as_str(),
            profile.digest.as_str(),
        );
        let wanted = (
            self.product.as_str(),
            self.configuration.generation.as_str(),
            self.profile.as_str(),
        );
        if standing != wanted {
            return Err(format!(
                "release marker {marker} Product Profile binding has drifted"
            ));
        }
        let spec = Spec::governed(root, target)?;
        Ok(Identity {
            product: spec.product.clone(),
            authority: spec.authority.clone(),
            configuration: Some(configuration),
            profile: Some(digest),
            spec: Box::new(spec),
        })
    }
}
