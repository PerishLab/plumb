use super::Descriptor;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(feature = "skill")]
const STABLE: &str = "https://releases.plumb.perish.uk";
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const REPOSITORY: &str = "PerishLab/plumb";
const VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expected {
    schema: String,
    tree: String,
    digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Authority {
    producer: String,
    depot: String,
}

#[derive(Debug)]
pub struct Verified(Descriptor);

#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    metadata: Metadata,
}

#[derive(Deserialize)]
struct Metadata {
    perish: Perish,
}

#[derive(Deserialize)]
pub(super) struct Perish {
    pub(super) guard: Guard,
    release: Option<Release>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Guard {
    schema: u64,
    depot: String,
}

#[cfg(feature = "skill")]
#[derive(Deserialize)]
struct Sealed {
    channel: String,
    #[serde(rename = "releaseVersion")]
    version: String,
    guard: Option<Stable>,
}

#[cfg(feature = "skill")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stable {
    producer: String,
    depot: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    schema: u64,
    repository: String,
    marker: String,
    commit: String,
    tree: String,
}

impl Expected {
    pub fn new(
        schema: impl Into<String>,
        tree: impl Into<String>,
        digest: impl Into<String>,
    ) -> Self {
        Self {
            schema: schema.into(),
            tree: tree.into(),
            digest: digest.into(),
        }
    }

    pub(super) fn held(proof: &Descriptor) -> Self {
        Self::new(&proof.schema, &proof.tree, &proof.digest)
    }
}

impl Authority {
    pub fn released() -> Result<Self, String> {
        Self::package(MANIFEST)
    }

    pub fn pinned() -> Result<String, String> {
        perish(MANIFEST).and_then(|perish| pin(perish.guard))
    }

    #[cfg(feature = "skill")]
    pub fn stable() -> Result<Self, String> {
        Self::published(STABLE)
    }

    pub fn running() -> Result<Self, String> {
        Ok(Self {
            producer: super::identity(),
            depot: crate::depot::rules()?.mark().to_string(),
        })
    }

    pub fn among(set: Vec<Self>, root: &Path, commit: &str) -> Result<Self, String> {
        let proof = super::commit(root, commit)?;
        let named = set
            .iter()
            .map(|authority| authority.producer.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        set.into_iter()
            .find(|authority| authority.producer == proof.plumb && authority.depot == proof.depot)
            .ok_or_else(|| {
                format!(
                    "guard proof used Plumb {} with depot {}, which no accepted authority names: [{named}]",
                    proof.plumb, proof.depot
                )
            })
    }

    pub fn producer(&self) -> &str {
        &self.producer
    }

    pub fn depot(&self) -> &str {
        &self.depot
    }

    pub fn verify(
        &self,
        root: &Path,
        commit: &str,
        expected: &Expected,
    ) -> Result<Verified, String> {
        let proof = super::commit(root, commit)?;
        self.judge(root, proof, expected)
    }

    #[cfg(test)]
    pub(super) fn fixture(producer: String, depot: &str) -> Self {
        Self {
            producer,
            depot: depot.into(),
        }
    }

    pub(super) fn judge(
        &self,
        root: &Path,
        proof: Descriptor,
        expected: &Expected,
    ) -> Result<Verified, String> {
        proof.validate()?;
        proof.repository(root)?;
        if proof.plumb != self.producer {
            return Err(format!(
                "guard proof used Plumb {}, not released authority {}",
                proof.plumb, self.producer
            ));
        }
        if proof.depot != self.depot {
            return Err(format!(
                "guard proof used depot {}, not released authority {}",
                proof.depot, self.depot
            ));
        }
        proof.platform()?;
        if (&proof.schema, &proof.tree, &proof.digest)
            != (&expected.schema, &expected.tree, &expected.digest)
        {
            return Err("guard proof does not match the expected schema, tree, and digest".into());
        }
        Ok(Verified(proof))
    }

    #[cfg(feature = "skill")]
    pub(super) fn published(base: &str) -> Result<Self, String> {
        let pointer = crate::seat::release::Authority::at(base).pointer("stable");
        let body = crate::skill::sealed(base).map_err(|error| {
            format!("cannot read Plumb stable Guard authority at {pointer}: {error}")
        })?;
        let seal: Sealed = serde_json::from_slice(&body)
            .map_err(|error| format!("stable seal named by {pointer} is malformed: {error}"))?;
        if seal.channel != "stable" {
            return Err(format!(
                "stable seal named by {pointer} is for channel {}",
                seal.channel
            ));
        }
        let guard = seal.guard.ok_or_else(|| {
            format!(
                "Plumb stable {} seal named by {pointer} carries no guard authority",
                seal.version
            )
        })?;
        let (marker, commit) = guard
            .producer
            .split_once('@')
            .unwrap_or((&guard.producer, ""));
        if marker != seal.version {
            return Err(format!(
                "Plumb stable {} seal names guard producer {}",
                seal.version, guard.producer
            ));
        }
        hash(commit, "producer commit")
            .and_then(|()| hash(&guard.depot, "Depot mark"))
            .map_err(|error| format!("Plumb stable {} seal: {error}", seal.version))?;
        Ok(Self {
            producer: guard.producer,
            depot: guard.depot,
        })
    }

    pub(super) fn package(text: &str) -> Result<Self, String> {
        let perish = perish(text)?;
        let depot = pin(perish.guard)?;
        let release = perish
            .release
            .ok_or("plumb-lib package carries no released authority")?;
        if release.schema != VERSION {
            return Err(format!(
                "plumb-lib release authority schema {} is not {VERSION}",
                release.schema
            ));
        }
        if release.repository != REPOSITORY {
            return Err(format!(
                "plumb-lib release authority names {}, not {REPOSITORY}",
                release.repository
            ));
        }
        let marker = release
            .marker
            .strip_prefix('v')
            .ok_or("plumb-lib release marker has no v prefix")?;
        let version = semver::Version::parse(marker)
            .map_err(|error| format!("plumb-lib release marker is invalid: {error}"))?;
        if version.to_string() != env!("CARGO_PKG_VERSION") {
            return Err(format!(
                "plumb-lib release marker {} does not match package v{}",
                release.marker,
                env!("CARGO_PKG_VERSION")
            ));
        }
        hash(&release.commit, "release commit")?;
        hash(&release.tree, "release tree")?;
        Ok(Self {
            producer: format!("{}@{}", release.marker, release.commit),
            depot,
        })
    }
}

impl Verified {
    pub fn descriptor(&self) -> &Descriptor {
        &self.0
    }

    pub fn take(self) -> Descriptor {
        self.0
    }
}

pub(super) fn perish(text: &str) -> Result<Perish, String> {
    toml::from_str::<Manifest>(text)
        .map(|manifest| manifest.package.metadata.perish)
        .map_err(|error| format!("cannot read plumb-lib package authority: {error}"))
}

pub(super) fn pin(guard: Guard) -> Result<String, String> {
    if guard.schema != VERSION {
        return Err(format!(
            "plumb-lib Guard authority schema {} is not {VERSION}",
            guard.schema
        ));
    }
    hash(&guard.depot, "Depot mark").map(|()| guard.depot)
}

fn hash(value: &str, name: &str) -> Result<(), String> {
    super::hash(value, name)
}
