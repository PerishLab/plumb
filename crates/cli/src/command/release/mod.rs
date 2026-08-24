pub(super) mod artifact;
pub(in crate::command) mod channel;
mod deed;
pub(super) mod manager;
pub(in crate::command) mod output;
mod plan;
mod truth;
pub(in crate::command) mod workspace;

pub use deed::Deed;
use plumb::rig::{Authority, Rig};
use std::path::{Path, PathBuf};
use truth::promotion;
pub(super) use truth::{proof, record, storage, verify};

use crate::shape::release::Spec;

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
        Deed::Stamp { version, dry } => super::operator::stamp(&version, dry),
        Deed::Retract { version, dry } => super::operator::retract(&version, dry),
        Deed::Rejoin { ref version, .. } if !version.is_empty() => super::operator::line(deed),
        Deed::Prepare { .. } | Deed::Pick { .. } | Deed::Freeze { .. } => {
            super::operator::line(deed)
        }
        deed => carry(deed),
    }
}

fn carry(deed: Deed) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let manifest = rig.release.root.join("plumb.toml");
    let spec = Spec::read(&manifest)?;
    let release = &rig.release;
    match deed {
        Deed::Plan => plan::plan(
            &spec,
            required("PLUMB_RELEASE_SOURCE", &release.source)?,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
        ),
        Deed::Surface => plan::surface(&spec),
        Deed::Activate => storage::activate(&capsule(release)?, &rig.activate),
        Deed::Compile => compile(&spec, release),
        Deed::Inspect => verify::inspect(
            required("PLUMB_RELEASE_URL", &release.url)?,
            release.activated,
        ),
        Deed::Stamp { .. } | Deed::Retract { .. } => {
            Err("a point verb does not read the release environment".into())
        }
        Deed::Prepare { .. } | Deed::Pick { .. } | Deed::Freeze { .. } => {
            Err("a line verb does not read the release environment".into())
        }
        Deed::Rejoin { .. } => crate::command::operator::topology::rejoin(
            &spec.root,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_BASE", &release.base)?,
        ),
        Deed::Promote => promotion::fetch(
            &spec,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            release
                .promotion
                .as_deref()
                .ok_or_else(|| "PLUMB_RELEASE_PROMOTION is required".to_string())?,
        ),
        Deed::Evidence => crate::command::operator::topology::evidence(
            crate::command::operator::topology::Guard {
                api: required("PLUMB_GUARD_API", &rig.guard.api)?,
                repository: required("PLUMB_GUARD_REPOSITORY", &rig.guard.repository)?,
                token: required("PLUMB_GUARD_TOKEN", &rig.guard.token)?,
                commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            },
            &rig.guard.contexts,
        ),
    }
}

fn compile(spec: &Spec, release: &plumb::rig::Release) -> Result<String, String> {
    let channel = required("PLUMB_RELEASE_CHANNEL", &release.channel)?;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let commit = required("PLUMB_RELEASE_COMMIT", &release.commit)?;
    output::capsule::compile(output::capsule::Compile {
        spec: &spec.manifest(),
        channel,
        version,
        commit,
        artifacts: &artifacts(release)?,
        out: &output(release)?,
        promotion: release.promotion.as_deref(),
        toolchain: &release.toolchain,
    })
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

pub(super) fn channel(version: &str) -> Result<String, String> {
    channel::channel(version)
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    Spec::read(&root.join("plumb.toml")).map(|spec| spec.authority)
}

pub(super) fn inspect(url: &str) -> Result<String, String> {
    verify::inspect(url, true)
}

pub(super) fn promotion(
    spec: &Spec,
    commit: &str,
    version: &str,
) -> Result<promotion::Exact, String> {
    promotion::derive(spec, commit, version)
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
