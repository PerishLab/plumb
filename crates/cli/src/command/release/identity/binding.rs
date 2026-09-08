use crate::shape::release::Spec;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Datum {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    sha256: String,
}

pub(super) fn datum(
    root: &Path,
    version: &str,
    commit: &str,
    expected: Option<&str>,
) -> Result<Datum, String> {
    if let Some(expected) = expected {
        let datum = plumb::datum::Git(root)
            .at(version, commit)?
            .ok_or("release marker has no commit-carried datum")?;
        let sha256 = datum.digest()?;
        if sha256 != expected {
            return Err("release marker datum digest has drifted".into());
        }
        return Ok(Datum {
            path: None,
            commit: Some(commit.to_string()),
            sha256,
        });
    }
    let path = plumb::datum::leaf(version);
    let output = super::seat::command(root, ["show", &format!("{commit}:{path}")])?;
    if !output.status.success() {
        return Err(format!("cannot read historical datum {path}"));
    }
    plumb::datum::decode(version, &output.stdout)?;
    Ok(Datum {
        path: Some(path),
        commit: None,
        sha256: super::super::record::sha(&output.stdout),
    })
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Annotation {
    schema: String,
    product: String,
    marker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    configuration: Option<Configuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    datum: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Configuration {
    channel: String,
    version: String,
    generation: String,
}

pub(super) struct Identity {
    pub schema: &'static str,
    pub product: String,
    pub authority: String,
    pub configuration: Option<String>,
    pub profile: Option<String>,
    pub datum: Option<String>,
    pub spec: Box<Spec>,
}

pub(super) fn annotation(spec: &Spec, marker: &str, head: &str) -> Result<String, String> {
    let base = marker.split('-').next().unwrap_or(marker);
    let datum = plumb::datum::Git(&spec.root)
        .at(base, head)?
        .ok_or("release marker requires commit-carried datum")?
        .digest()?;
    let (Some(configuration), Some(profile)) = (&spec.configuration, &spec.profile) else {
        return serde_json::to_string(&Annotation {
            schema: "plumb.release-marker/v4".into(),
            product: spec.product.clone(),
            marker: marker.into(),
            configuration: None,
            profile: None,
            datum: Some(datum),
        })
        .map_err(|error| error.to_string());
    };
    let rules = plumb::depot::rules()?;
    if rules.mark() != configuration {
        return Err("release marker configuration changed while it was being stamped".into());
    }
    let version = rules
        .version()
        .ok_or_else(|| "a Product Profile requires version-related configuration".to_string())?;
    let channel = rules
        .channel()
        .ok_or_else(|| "a Product Profile requires a v3 configuration generation".to_string())?;
    serde_json::to_string(&Annotation {
        schema: "plumb.release-marker/v4".into(),
        product: spec.product.clone(),
        marker: marker.to_string(),
        configuration: Some(Configuration {
            channel: channel.to_string(),
            version: version.to_string(),
            generation: configuration.clone(),
        }),
        profile: Some(profile.clone()),
        datum: Some(datum),
    })
    .map_err(|error| format!("cannot encode release marker annotation: {error}"))
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
    fn protocol(&self) -> Result<&'static str, String> {
        Ok(match self.schema.as_str() {
            "plumb.release-marker/v2" => "plumb.release-marker/v2",
            "plumb.release-marker/v3" => "plumb.release-marker/v3",
            "plumb.release-marker/v4" => "plumb.release-marker/v4",
            _ => {
                return Err(format!(
                    "unknown release marker annotation schema {}",
                    self.schema
                ));
            }
        })
    }

    fn resolve(self, root: &Path, marker: &str) -> Result<Identity, String> {
        let schema = self.protocol()?;
        if self.marker != marker {
            return Err(format!(
                "release marker {marker} annotation names {}",
                self.marker
            ));
        }
        if (schema == "plumb.release-marker/v4") != self.datum.is_some() {
            return Err("release marker datum carrier disagrees with its schema".into());
        }
        let Some(configuration) = &self.configuration else {
            if schema != "plumb.release-marker/v4" || self.profile.is_some() {
                return Err("release marker has incomplete configuration identity".into());
            }
            let spec = Spec::resolve(root)?;
            if spec.product != self.product || spec.profile.is_some() {
                return Err("release marker omitted its Product Profile binding".into());
            }
            return Ok(Identity {
                schema,
                product: spec.product.clone(),
                authority: spec.authority.clone(),
                configuration: None,
                profile: None,
                datum: self.datum,
                spec: Box::new(spec),
            });
        };
        let expected = self
            .profile
            .as_deref()
            .ok_or("release marker has no profile")?;
        let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
        let seat = plumb::depot::Rules::exact(
            &plumb::depot::root(Path::new(""))?,
            plumb::depot::Selection {
                source: &rig.rules.source,
                channel: &configuration.channel,
                version: &configuration.version,
                generation: &configuration.generation,
            },
        )?;
        let target = crate::shape::product::at(root, &rig.rules.source, &seat)?;
        let profile = target
            .profile
            .as_ref()
            .ok_or_else(|| "release marker configuration carries no Product Profile".to_string())?;
        let generation = profile.configuration.clone();
        let digest = profile.digest.clone();
        let standing = (
            target.product.as_str(),
            profile.configuration.as_str(),
            profile.digest.as_str(),
        );
        let wanted = (
            self.product.as_str(),
            configuration.generation.as_str(),
            expected,
        );
        if standing != wanted {
            return Err(format!(
                "release marker {marker} Product Profile binding has drifted"
            ));
        }
        let spec = Spec::governed(root, target)?;
        Ok(Identity {
            schema,
            product: spec.product.clone(),
            authority: spec.authority.clone(),
            configuration: Some(generation),
            profile: Some(digest),
            datum: self.datum,
            spec: Box::new(spec),
        })
    }
}

pub(super) fn protocol(message: &str) -> Result<Option<&'static str>, String> {
    if !message.starts_with('{') {
        return Ok(None);
    }
    let annotation: Annotation = serde_json::from_str(message)
        .map_err(|error| format!("cannot parse release marker annotation: {error}"))?;
    annotation.protocol().map(Some)
}
