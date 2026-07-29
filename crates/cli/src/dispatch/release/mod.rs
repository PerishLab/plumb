mod capsule;
mod engine;
mod manager;
mod model;
mod proof;
mod record;
mod smoke;
mod storage;
mod verify;

use clap::Subcommand;
use plumb::rig::{Authority, Rig};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Deed {
    Activate,
    Assemble,
    Build,
    Compile,
    Inspect,
    Managers,
    Matrix,
    Promote,
    Publish,
    Registry {
        #[command(subcommand)]
        deed: Registry,
    },
    Smoke,
    Tag,
    Verify,
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
        Deed::Assemble => engine::package::product(&spec).assemble(
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &artifacts(release)?,
        ),
        Deed::Build => engine::package::product(&spec).build(engine::package::Build {
            target: required("PLUMB_RELEASE_TARGET", &release.target)?,
            version: required("PLUMB_RELEASE_VERSION", &release.version)?,
            channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            artifacts: &artifacts(release)?,
        }),
        Deed::Compile => compile(&spec, release),
        Deed::Inspect => verify::inspect(
            required("PLUMB_RELEASE_URL", &release.url)?,
            release.activated,
        ),
        Deed::Managers => manager::write(
            &manifest,
            required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &output(release)?,
        ),
        Deed::Matrix => engine::package::product(&spec).matrix(),
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
        Deed::Publish => storage::publish(&capsule(release)?, &rig.publish),
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
        Deed::Smoke => smoke::run(
            &manifest,
            required("PLUMB_RELEASE_URL", &release.url)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
        ),
        Deed::Tag => engine::tag::run(
            &spec.root,
            required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
        ),
        Deed::Verify => verify::run(&capsule(release)?, release.activated),
    }
}

fn compile(spec: &model::Spec, release: &plumb::rig::Release) -> Result<String, String> {
    let channel = required("PLUMB_RELEASE_CHANNEL", &release.channel)?;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    if channel == "stable" {
        let missing = crate::shape::changelog::read(&spec.root, version);
        if !missing.is_empty() {
            return Err(format!(
                "stable changelog is incomplete at {}: {}",
                crate::shape::changelog::seat(&spec.root, version).display(),
                missing.join("; ")
            ));
        }
    }
    capsule::compile(capsule::Compile {
        spec: &spec.manifest(),
        channel,
        version,
        commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
        artifacts: &artifacts(release)?,
        out: &output(release)?,
        promotion: release.promotion.as_deref(),
    })
}

fn artifacts(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if !release.artifacts.as_os_str().is_empty() {
        return Ok(rebase(&release.root, &release.artifacts));
    }
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    Ok(release.root.join("dist").join(version))
}

fn output(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if release.output.as_os_str().is_empty() {
        return Err("PLUMB_RELEASE_OUTPUT is required".into());
    }
    Ok(rebase(&release.root, &release.output))
}

fn capsule(release: &plumb::rig::Release) -> Result<PathBuf, String> {
    if !release.capsule.as_os_str().is_empty() {
        return Ok(rebase(&release.root, &release.capsule));
    }
    Ok(output(release)?.join("capsule.json"))
}

fn required<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
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
