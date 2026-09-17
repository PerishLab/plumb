pub(super) mod artifact;
pub mod authority;
pub(in crate::command) mod channel;
mod identity;
pub(in crate::command) mod output;
pub(in crate::command) mod plan;
mod truth;
pub(in crate::command) mod workspace;

pub use identity::deed::Deed;
pub(in crate::command) use identity::marker::Descriptor as ReleaseMarker;
use identity::{marker as markers, promotion};
use plumb::rig::Authority;
use std::path::{Path, PathBuf};
pub(super) use truth::{manager, projection, proof, record, storage, verify};

use crate::shape::release::Spec;

pub(in crate::command) struct Product<'a>(&'a Spec);

impl<'a> Product<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Self(spec)
    }

    pub fn compile(&self, release: &plumb::rig::Release) -> Result<String, String> {
        let channel = required("PLUMB_RELEASE_CHANNEL", &release.channel)?;
        let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
        let commit = required("PLUMB_RELEASE_COMMIT", &release.commit)?;
        output::capsule::compile(output::capsule::Compile {
            spec: self.0,
            channel,
            version,
            commit,
            artifacts: &artifacts(release)?,
            out: &output(release)?,
            promotion: (channel == "stable")
                .then_some(release.promotion.as_deref())
                .flatten(),
            toolchain: &release.toolchain,
        })
    }

    pub fn promote(&self, release: &plumb::rig::Release) -> Result<String, String> {
        let channel = required("PLUMB_RELEASE_CHANNEL", &release.channel)?;
        if channel != "stable" {
            return Ok(format!("{channel} carries no promotion proof"));
        }
        promotion::Promotion::new(self.0).fetch(
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            release
                .promotion
                .as_deref()
                .ok_or_else(|| "PLUMB_RELEASE_PROMOTION is required".to_string())?,
        )
    }

    pub fn depot(&self) -> truth::depot::Source<'_> {
        truth::depot::Source {
            product: &self.0.product,
            authority: &self.0.authority,
        }
    }

    pub fn activated(&self) -> Result<Option<(String, String)>, String> {
        let url = format!("{}/v1/channels/stable.json", self.0.authority);
        if verify::optional(&url)?.is_none() {
            return Ok(None);
        }
        let binding = self.depot().latest("stable", false)?;
        Ok(Some((binding.release.version, binding.release.commit)))
    }

    pub fn promotion(&self, commit: &str, version: &str) -> Result<promotion::Exact, String> {
        promotion::Promotion::new(self.0).derive(commit, version)
    }
}

pub(in crate::command) fn marker(raw: &str) -> Result<ReleaseMarker, String> {
    markers::resolve(raw, true)
}

pub(in crate::command) fn snapshot(raw: &str) -> Result<ReleaseMarker, String> {
    markers::resolve(raw, false)
}

pub(in crate::command) fn annotation(spec: &Spec, marker: &str) -> Result<String, String> {
    markers::annotation(spec, marker)
}

impl ReleaseMarker {
    pub(in crate::command) fn bound(root: &Path, raw: &str) -> Result<Self, String> {
        markers::bound(root, raw)
    }
}

pub fn run(deed: Deed) -> i32 {
    let result = execute(deed);
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb release: {error}");
            1
        }
    }
}

fn execute(deed: Deed) -> Result<String, String> {
    match deed {
        Deed::Stamp {
            version,
            remote,
            dry,
        } => super::operator::stamp(&version, &remote, dry),
        Deed::Retract { version, dry } => super::operator::retract(&version, dry),
        deed => markers::run(deed),
    }
}

pub(super) fn artifacts(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if !release.artifacts.as_os_str().is_empty() {
        return Ok(rebase(&release.root, &release.artifacts));
    }
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    Ok(release.root.join("dist").join(version))
}

pub(super) fn output(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if release.output.as_os_str().is_empty() {
        return Err("PLUMB_RELEASE_OUTPUT is required".into());
    }
    Ok(rebase(&release.root, &release.output))
}

pub(super) fn capsule(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if !release.capsule.as_os_str().is_empty() {
        return Ok(rebase(&release.root, &release.capsule));
    }
    Ok(output(release)?.join("capsule.json"))
}

pub(super) fn required<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.trim().is_empty() {
        Err(format!("{name} is required"))
    } else {
        Ok(value)
    }
}

fn rebase(root: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        root.join(path)
    } else {
        path.to_path_buf()
    }
}

pub(crate) fn channel(version: &str) -> Result<String, String> {
    channel::channel(version)
}

pub(in crate::command) use truth::depot::validate as validate_depot;

pub(in crate::command) fn knowledge<'a>(
    product: &'a str,
    authority: &'a str,
) -> truth::depot::Source<'a> {
    truth::depot::Source { product, authority }
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    Spec::controller(root).map(|spec| spec.authority)
}

pub(super) fn inspect(url: &str) -> Result<String, String> {
    verify::inspect(url, true)
}

pub(super) fn settled(root: &Path, version: &str, commit: &str) -> Result<String, String> {
    crate::command::operator::topology::rejoin(root, version, commit, "origin/main")
}

impl storage::Authority for Authority {
    fn access(&self) -> &str {
        &self.access
    }

    fn secret(&self) -> &str {
        &self.secret
    }

    fn bucket(&self) -> &str {
        &self.bucket
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
