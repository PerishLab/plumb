mod artifact;
mod capsule;
pub(super) mod engine;
pub(crate) mod generator;
pub(super) mod manager;
pub(crate) mod model;
pub(super) mod object;
mod proof;
pub(super) mod record;
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
    Evidence,
    Inspect,
    Rejoin,
    Promote,
    Reference,
    Source,
    Surface,
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
        Deed::Surface => surface(&spec),
        Deed::Rejoin => engine::topology::rejoin(
            &spec.root,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_BASE", &release.base)?,
        ),
        Deed::Promote => engine::promotion::fetch(
            &spec,
            required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            release
                .promotion
                .as_deref()
                .ok_or_else(|| "PLUMB_RELEASE_PROMOTION is required".to_string())?,
        ),
        Deed::Evidence => engine::topology::evidence(
            engine::topology::Guard {
                api: required("PLUMB_GUARD_API", &rig.guard.api)?,
                repository: required("PLUMB_GUARD_REPOSITORY", &rig.guard.repository)?,
                token: required("PLUMB_GUARD_TOKEN", &rig.guard.token)?,
                commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            },
            &rig.guard.contexts,
        ),
        Deed::Reference => {
            engine::topology::reference(required("PLUMB_RELEASE_SOURCE", &release.source)?)
        }
        Deed::Source => engine::topology::source(engine::topology::Source {
            root: &spec.root,
            channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            version: required("PLUMB_RELEASE_VERSION", &release.version)?,
            commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            reference: required("PLUMB_RELEASE_SOURCE", &release.source)?,
        }),
    }
}

fn prepared(medium: &str) -> Result<&'static str, String> {
    match medium {
        "cargo" | "cfworker" => Ok("rehearse"),
        "chart" => Ok("package"),
        "npm" => Ok("pack"),
        "oci" => Ok("build"),
        _ => Err(format!("{medium} names no deed that prepares it")),
    }
}

fn surface(spec: &model::Spec) -> Result<String, String> {
    let media = spec.surface();
    let row = |medium: &&str| serde_json::json!({ "medium": medium });
    let include = media.iter().map(row).collect::<Vec<_>>();
    let project = media
        .iter()
        .filter(|medium| **medium != "binary")
        .map(|medium| {
            prepared(medium)
                .map(|prepare| serde_json::json!({ "medium": medium, "prepare": prepare }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let seal = media
        .iter()
        .filter(|medium| **medium == "binary")
        .map(|_| serde_json::json!({ "held": "seal" }))
        .collect::<Vec<_>>();
    serde_json::to_string(&serde_json::json!({
        "include": include,
        "project": { "include": project },
        "seal": { "include": seal },
    }))
    .map_err(|error| error.to_string())
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
    manager::channel(version)
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    model::Spec::read(&root.join("plumb.toml")).map(|spec| spec.authority)
}

pub(super) fn inspect(url: &str) -> Result<String, String> {
    verify::inspect(url, true)
}

pub(super) fn promotion(
    spec: &model::Spec,
    commit: &str,
    version: &str,
) -> Result<engine::promotion::Exact, String> {
    engine::promotion::derive(spec, commit, version)
}

pub(super) fn settled(root: &Path, version: &str, commit: &str) -> Result<String, String> {
    engine::topology::rejoin(root, version, commit, "origin/main")
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
