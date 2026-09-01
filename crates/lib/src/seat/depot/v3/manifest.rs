use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::value::Value;
use super::{FORMAT, Identity, Kind, Marker, Object};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub product: String,
    pub channel: String,
    pub version: String,
    pub marker: Marker,
    pub kind: Kind,
    pub objects: Vec<Object>,
}

impl Manifest {
    pub fn new(identity: Identity, mut objects: Vec<Object>) -> Result<Self, String> {
        objects.sort();
        let held = Self {
            format: FORMAT,
            product: identity.product,
            channel: identity.channel,
            version: identity.version,
            marker: identity.marker,
            kind: identity.kind,
            objects,
        };
        held.validate()?;
        Ok(held)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let held: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("cannot parse a depot v3 manifest: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec_pretty(self)
            .map_err(|error| format!("cannot encode a depot v3 manifest: {error}"))
    }

    pub fn generation(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(|bytes| super::super::sha(&bytes))
            .map_err(|error| format!("cannot identify a depot generation: {error}"))
    }

    pub fn verify(&self, path: &str, bytes: &[u8], executable: bool) -> Result<(), String> {
        Value(path).anchored("depot object path")?;
        let object = self
            .objects
            .iter()
            .find(|held| held.path == path)
            .ok_or_else(|| format!("depot manifest names no object at {path}"))?;
        if object.sha256 != super::super::sha(bytes)
            || object.size != bytes.len() as u64
            || object.executable != executable
        {
            return Err(format!("depot object drift: {path}"));
        }
        Ok(())
    }

    pub fn identity(&self) -> Identity {
        Identity {
            product: self.product.clone(),
            channel: self.channel.clone(),
            version: self.version.clone(),
            marker: self.marker.clone(),
            kind: self.kind,
        }
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "depot manifest format must be {FORMAT}, got {}",
                self.format
            ));
        }
        self.identity().validate()?;
        if self.objects.is_empty() {
            return Err("depot manifest carries no objects".into());
        }
        let mut paths = BTreeSet::new();
        for object in &self.objects {
            object.validate()?;
            if !paths.insert(&object.path) {
                return Err(format!("depot manifest repeats {}", object.path));
            }
        }
        if !self.objects.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("depot manifest objects are not canonically ordered".into());
        }
        Ok(())
    }
}

impl Identity {
    pub(super) fn validate(&self) -> Result<(), String> {
        Value(&self.product).component("depot product")?;
        Value(&self.channel).component("release channel")?;
        super::value::version(&self.version)?;
        self.marker.validate()?;
        if self.marker.name != self.version {
            return Err(format!(
                "release marker {} does not bind version {}",
                self.marker.name, self.version
            ));
        }
        Ok(())
    }
}

impl Marker {
    fn validate(&self) -> Result<(), String> {
        Value(&self.name).component("release marker")?;
        Value(&self.sha256).digest("release marker sha256")
    }
}

impl Object {
    fn validate(&self) -> Result<(), String> {
        Value(&self.path).anchored("depot object path")?;
        Value(&self.sha256).digest("depot object sha256")?;
        if !self.media.contains('/') || self.media.chars().any(char::is_whitespace) {
            return Err(format!("invalid depot object media type: {}", self.media));
        }
        Ok(())
    }
}
