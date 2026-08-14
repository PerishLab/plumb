mod artifact;
mod capsule;
pub(super) mod engine;
pub(super) mod generator;
pub(super) mod manager;
pub(super) mod model;
mod proof;
mod record;
pub(super) mod smoke;
pub(super) mod storage;
pub(super) mod verify;

use clap::Subcommand;
use plumb::rig::{Authority, Rig};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Deed {
    Activate,
    Authority,
    Channel,
    Compile,
    Inspect,
    Packport,
    Promote,
    Registry {
        #[command(subcommand)]
        deed: Registry,
    },
    Source,
}

#[derive(Subcommand)]
pub enum Registry {
    Publish,
    Rehearse,
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
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let manifest = rig.release.root.join("plumb.toml");
    let spec = model::Spec::read(&manifest)?;
    let release = &rig.release;
    match deed {
        Deed::Activate => storage::activate(&capsule(release)?, &rig.activate),
        Deed::Authority => Ok(spec.authority.clone()),
        Deed::Channel => manager::channel(required("PLUMB_RELEASE_VERSION", &release.version)?),
        Deed::Compile => compile(&spec, release),
        Deed::Inspect => verify::inspect(
            required("PLUMB_RELEASE_URL", &release.url)?,
            release.activated,
        ),
        Deed::Packport => engine::topology::packport(
            &spec.root,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_BASE", &release.base)?,
        ),
        Deed::Promote => engine::promotion::fetch(
            &spec,
            required(
                "PLUMB_RELEASE_PROMOTION_CHANNEL",
                &release.promotion_channel,
            )?,
            required(
                "PLUMB_RELEASE_PROMOTION_VERSION",
                &release.promotion_version,
            )?,
            release
                .promotion
                .as_deref()
                .ok_or_else(|| "PLUMB_RELEASE_PROMOTION is required".to_string())?,
        ),
        Deed::Registry { deed } => match deed {
            Registry::Publish => engine::registry::registry(&spec).publish(
                required("PLUMB_RELEASE_VERSION", &release.version)?,
                &release.registry_token,
            ),
            Registry::Rehearse => engine::registry::registry(&spec).rehearse(
                required("PLUMB_RELEASE_VERSION", &release.version)?,
                &release.registry_token,
            ),
        },
        Deed::Source => engine::topology::source(engine::topology::Source {
            root: &spec.root,
            channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            version: required("PLUMB_RELEASE_VERSION", &release.version)?,
            commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            reference: required("PLUMB_RELEASE_SOURCE", &release.source)?,
        }),
    }
}

fn compile(spec: &model::Spec, release: &plumb::rig::Release) -> Result<String, String> {
    let channel = required("PLUMB_RELEASE_CHANNEL", &release.channel)?;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let commit = required("PLUMB_RELEASE_COMMIT", &release.commit)?;
    let previous = crate::shape::changelog::previous(&spec.authority)?;
    let changelog =
        crate::shape::changelog::prove(&spec.root, version, previous.as_deref(), commit)?;
    capsule::compile(capsule::Compile {
        spec: &spec.manifest(),
        channel,
        version,
        commit,
        artifacts: &artifacts(release)?,
        out: &output(release)?,
        promotion: release.promotion.as_deref(),
        changelog,
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
    manager::channel(version)
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    model::Spec::read(&root.join("plumb.toml")).map(|spec| spec.authority)
}

pub(super) fn inspect(url: &str) -> Result<String, String> {
    verify::inspect(url, true)
}

pub(super) fn settled(root: &Path, version: &str, commit: &str) -> Result<String, String> {
    engine::topology::packport(root, version, commit, "origin/main")
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
