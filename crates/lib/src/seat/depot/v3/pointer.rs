use serde::{Deserialize, Serialize};

use super::value::{Value, timestamp};
use super::{FORMAT, Identity, LEAF, Manifest, Marker, Publication, Reference, Route};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Pointer {
    pub format: u32,
    pub product: String,
    pub channel: String,
    pub version: String,
    pub marker: Marker,
    pub kind: super::Kind,
    pub generation: String,
    pub manifest: Reference,
    #[serde(rename = "previousGeneration")]
    pub prior: Option<String>,
    #[serde(rename = "createdAt")]
    pub created: String,
}

#[derive(Eq, PartialEq)]
struct Projection<'a> {
    product: &'a str,
    channel: &'a str,
    version: &'a str,
    kind: super::Kind,
}

impl Pointer {
    pub fn new(held: &Manifest, publication: Publication<'_>) -> Result<Self, String> {
        let generation = held.generation()?;
        let bytes = held.encode()?;
        let route = Route::new(&held.channel, held.kind, &held.version);
        let url = super::manifest(publication.source, route, &generation)?;
        let pointer = Self {
            format: FORMAT,
            product: held.product.clone(),
            channel: held.channel.clone(),
            version: held.version.clone(),
            marker: held.marker.clone(),
            kind: held.kind,
            generation,
            manifest: Reference {
                url,
                sha256: super::super::sha(&bytes),
                size: bytes.len() as u64,
            },
            prior: publication.prior,
            created: publication.created,
        };
        pointer.validate()?;
        Ok(pointer)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let held: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("cannot parse a depot v3 pointer: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec_pretty(self)
            .map_err(|error| format!("cannot encode a depot v3 pointer: {error}"))
    }

    pub fn bind(&self, held: &Manifest, bytes: &[u8]) -> Result<(), String> {
        held.validate()?;
        if self.identity() != held.identity()
            || self.generation != held.generation()?
            || self.manifest.sha256 != super::super::sha(bytes)
            || self.manifest.size != bytes.len() as u64
        {
            return Err(format!(
                "depot pointer {} does not bind its manifest",
                self.generation
            ));
        }
        Ok(())
    }

    pub fn advance(&self, next: &Self) -> Result<bool, String> {
        self.validate()?;
        next.validate()?;
        if self.identity() != next.identity() {
            if self.projection() != next.projection() {
                return Err("standing depot pointer names another marker projection".into());
            }
            if next.prior.is_some() {
                return Err("a rebound depot marker must begin a new generation lineage".into());
            }
            return Ok(true);
        }
        if self.generation == next.generation {
            return Ok(false);
        }
        if next.prior.as_deref() != Some(&self.generation) {
            return Err("depot latest does not continue from its standing generation".into());
        }
        Ok(true)
    }

    pub fn projects(&self, held: &Manifest) -> bool {
        self.identity() == held.identity()
    }

    fn projection(&self) -> Projection<'_> {
        Projection {
            product: &self.product,
            channel: &self.channel,
            version: &self.version,
            kind: self.kind,
        }
    }

    fn identity(&self) -> Identity {
        Identity {
            product: self.product.clone(),
            channel: self.channel.clone(),
            version: self.version.clone(),
            marker: self.marker.clone(),
            kind: self.kind,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "depot pointer format must be {FORMAT}, got {}",
                self.format
            ));
        }
        self.identity().validate()?;
        Value(&self.generation).digest("depot generation")?;
        Value(&self.manifest.url).source()?;
        Value(&self.manifest.sha256).digest("depot manifest sha256")?;
        let route = Route::new(&self.channel, self.kind, &self.version);
        let suffix = format!("/{}/{LEAF}", super::generation(route, &self.generation)?);
        if !self.manifest.url.ends_with(&suffix) {
            return Err("depot pointer manifest URL names another generation".into());
        }
        if let Some(previous) = &self.prior {
            Value(previous).digest("previous depot generation")?;
        }
        timestamp(&self.created)
    }
}
