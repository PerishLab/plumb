use serde::{Deserialize, Serialize};
#[cfg(feature = "depot")]
use std::collections::BTreeMap;
#[cfg(feature = "depot")]
use std::path::Path;

#[cfg(feature = "depot")]
use super::remote::{pull, read};
use super::value::Value;
#[cfg(feature = "depot")]
use super::{Bundle, Cache};
use super::{Kind, LEAF, Manifest, Pointer, Reference, Route};

const CAP: u64 = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pinned {
    pub channel: String,
    pub generation: String,
    pub manifest: Reference,
    pub version: String,
}

#[derive(Serialize)]
struct Wire<'a> {
    channel: &'a str,
    generation: &'a str,
    manifest: Record<'a>,
    version: &'a str,
}

#[derive(Serialize)]
struct Record<'a> {
    sha256: &'a str,
    size: u64,
    url: &'a str,
}

#[derive(Eq, PartialEq)]
struct Binding<'a> {
    product: &'a str,
    channel: &'a str,
    version: &'a str,
    kind: Kind,
    generation: String,
    size: u64,
    sha256: String,
}

impl Pinned {
    pub fn pointer(pointer: &Pointer) -> Result<Self, String> {
        if pointer.product != "plumb"
            || pointer.channel != "stable"
            || pointer.kind != Kind::Configuration
        {
            return Err(format!(
                "pinned depot configuration requires plumb/stable/configuration, got {}/{}/{}",
                pointer.product,
                pointer.channel,
                pointer.kind.label()
            ));
        }
        let held = Self {
            channel: pointer.channel.clone(),
            generation: pointer.generation.clone(),
            manifest: pointer.manifest.clone(),
            version: pointer.version.clone(),
        };
        let origin = held.fields()?;
        held.validate(&origin)?;
        Ok(held)
    }

    pub fn parse(raw: &str, source: &str) -> Result<Self, String> {
        let held: Self = serde_json::from_str(raw)
            .map_err(|error| format!("cannot parse pinned depot configuration: {error}"))?;
        held.validate(source)?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        self.fields()?;
        let wire = Wire {
            channel: &self.channel,
            generation: &self.generation,
            manifest: Record {
                sha256: &self.manifest.sha256,
                size: self.manifest.size,
                url: &self.manifest.url,
            },
            version: &self.version,
        };
        serde_json::to_string(&wire)
            .map_err(|error| format!("cannot encode pinned depot configuration: {error}"))
    }

    pub fn validate(&self, source: &str) -> Result<(), String> {
        Value(source).source()?;
        self.fields()?;
        let route = Route::new(&self.channel, Kind::Configuration, &self.version);
        let expected = super::manifest(source, route, &self.generation)?;
        if self.manifest.url != expected {
            return Err("pinned depot configuration names another source".into());
        }
        Ok(())
    }

    pub fn bind(&self, held: &Manifest, bytes: &[u8]) -> Result<(), String> {
        self.fields()?;
        held.validate()?;
        let actual = Binding {
            product: held.product.as_str(),
            channel: held.channel.as_str(),
            version: held.version.as_str(),
            kind: held.kind,
            generation: held.generation()?,
            size: bytes.len() as u64,
            sha256: super::super::sha(bytes),
        };
        let expected = Binding {
            product: "plumb",
            channel: "stable",
            version: self.version.as_str(),
            kind: Kind::Configuration,
            generation: self.generation.clone(),
            size: self.manifest.size,
            sha256: self.manifest.sha256.clone(),
        };
        if actual != expected {
            return Err(format!(
                "pinned depot configuration {} does not bind its manifest",
                self.generation
            ));
        }
        Ok(())
    }

    #[cfg(feature = "depot")]
    pub fn cached(&self, home: &Path, source: &str) -> Result<Bundle, String> {
        self.validate(source)?;
        let cache = Cache::new(home);
        if cache.has(&self.generation)? {
            return cache.pinned(self);
        }
        let (bundle, raw) = self.download()?;
        cache.keep(self, &raw, &bundle)?;
        Ok(bundle)
    }

    #[cfg(feature = "depot")]
    fn download(&self) -> Result<(Bundle, Vec<u8>), String> {
        let raw = pull(&self.manifest.url)?
            .ok_or_else(|| format!("depot generation {} has no manifest", self.generation))?;
        if raw.len() as u64 != self.manifest.size {
            return Err(format!(
                "pinned depot configuration {} manifest size disagrees",
                self.generation
            ));
        }
        let manifest = Manifest::parse(&raw)?;
        self.bind(&manifest, &raw)?;
        let base = self
            .manifest
            .url
            .strip_suffix(LEAF)
            .expect("a valid depot manifest URL ends in its leaf");
        let mut bodies = BTreeMap::new();
        for object in &manifest.objects {
            let bytes = read(base, &manifest, &object.path)?;
            bodies.insert(object.path.clone(), bytes);
        }
        Ok((Bundle { manifest, bodies }, raw))
    }

    fn fields(&self) -> Result<String, String> {
        if self.channel != "stable" {
            return Err(format!(
                "pinned depot configuration channel must be stable, got {}",
                self.channel
            ));
        }
        super::value::channel("stable", &self.version)?;
        Value(&self.generation).digest("depot generation")?;
        Value(&self.manifest.sha256).digest("depot manifest sha256")?;
        if self.manifest.size == 0 || self.manifest.size > CAP {
            return Err(format!(
                "pinned depot configuration manifest size out of bounds: {}",
                self.manifest.size
            ));
        }
        Value(&self.manifest.url).source()?;
        let route = Route::new(&self.channel, Kind::Configuration, &self.version);
        let suffix = format!("/{}/{LEAF}", super::generation(route, &self.generation)?);
        let origin = self
            .manifest
            .url
            .strip_suffix(&suffix)
            .ok_or_else(|| "pinned depot manifest URL names another generation".to_string())?;
        Value(origin).source()?;
        Ok(origin.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depot::sha;
    use crate::depot::v3::{Identity, Marker, Object, Publication};

    fn fixture(source: &str) -> (Manifest, Pointer, Pinned) {
        let manifest = Manifest::new(
            Identity {
                product: "plumb".into(),
                channel: "stable".into(),
                version: "v0.163.0".into(),
                marker: Marker {
                    name: "v0.163.0".into(),
                    sha256: "a".repeat(64),
                },
                kind: Kind::Configuration,
            },
            vec![Object {
                path: "rules/catalog.toml".into(),
                sha256: sha(b"answer = 42\n"),
                size: 12,
                media: "application/toml".into(),
                executable: false,
            }],
        )
        .expect("manifest");
        let pointer = Pointer::new(
            &manifest,
            Publication {
                source,
                prior: None,
                created: "2026-10-28T00:00:00Z".into(),
            },
        )
        .expect("pointer");
        let pinned = Pinned::pointer(&pointer).expect("pinned");
        (manifest, pointer, pinned)
    }

    #[test]
    fn canonical() {
        let source = "https://depot.plumb.perish.uk";
        let (manifest, _, pinned) = fixture(source);
        let encoded = pinned.encode().expect("encode");
        let expected = format!(
            "{{\"channel\":\"stable\",\"generation\":\"{}\",\"manifest\":{{\"sha256\":\"{}\",\"size\":{},\"url\":\"{}\"}},\"version\":\"v0.163.0\"}}",
            pinned.generation, pinned.manifest.sha256, pinned.manifest.size, pinned.manifest.url
        );
        assert_eq!(encoded, expected);
        let parsed = Pinned::parse(&encoded, source).expect("parse");
        assert_eq!(parsed, pinned);
        parsed
            .bind(&manifest, &manifest.encode().expect("raw"))
            .expect("bind");
    }

    #[test]
    fn rejected() {
        let source = "https://depot.plumb.perish.uk";
        let (_, _, pinned) = fixture(source);
        let encoded = pinned.encode().expect("encode");
        assert!(Pinned::parse(&encoded, "https://depot.other.perish.uk").is_err());
        let unknown = encoded.replacen('{', "{\"extra\":1,", 1);
        assert!(Pinned::parse(&unknown, source).is_err());
        let mut wrong = pinned.clone();
        wrong.channel = "rc".into();
        assert!(wrong.validate(source).is_err());
        let mut wrong = pinned.clone();
        wrong.version = "v0.163.0-rc.1".into();
        assert!(wrong.validate(source).is_err());
        let mut wrong = pinned.clone();
        wrong.manifest.size = 0;
        assert!(wrong.validate(source).is_err());
        let mut wrong = pinned;
        wrong.manifest.size = CAP + 1;
        assert!(wrong.validate(source).is_err());
    }
}
