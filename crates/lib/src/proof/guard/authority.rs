use super::Descriptor;
use serde::Deserialize;
use std::path::Path;

const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const REPOSITORY: &str = "PerishLab/plumb";
const VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expected {
    schema: String,
    tree: String,
    digest: String,
}

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
struct Perish {
    guard: Guard,
    release: Option<Release>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Guard {
    schema: u64,
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

    pub fn verify(
        &self,
        root: &Path,
        commit: &str,
        expected: &Expected,
    ) -> Result<Verified, String> {
        let proof = super::commit(root, commit)?;
        self.judge(root, proof, expected)
    }

    pub(crate) fn running() -> Result<Self, String> {
        Ok(Self {
            producer: super::identity(),
            depot: crate::depot::rules()?.mark().to_string(),
        })
    }

    #[cfg(test)]
    pub(super) fn fixture(producer: String, depot: &str) -> Self {
        Self {
            producer,
            depot: depot.into(),
        }
    }

    #[cfg(test)]
    pub(super) fn identity(&self) -> &str {
        &self.producer
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

    pub(super) fn package(text: &str) -> Result<Self, String> {
        let manifest: Manifest = toml::from_str(text)
            .map_err(|error| format!("cannot read plumb-lib package authority: {error}"))?;
        let guard = manifest.package.metadata.perish.guard;
        if guard.schema != VERSION {
            return Err(format!(
                "plumb-lib Guard authority schema {} is not {VERSION}",
                guard.schema
            ));
        }
        hash(&guard.depot, "Depot mark")?;
        let release = manifest
            .package
            .metadata
            .perish
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
            depot: guard.depot,
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

fn hash(value: &str, name: &str) -> Result<(), String> {
    super::hash(value, name)
}
