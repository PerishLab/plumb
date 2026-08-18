use super::release::{
    artifacts, capsule, engine, manager, output, required, smoke, storage, verify,
};
use clap::Subcommand;
use plumb::rig::Rig;

#[derive(Subcommand)]
pub enum Deed {
    Binary {
        #[command(subcommand)]
        deed: Binary,
    },
    Cargo {
        #[command(subcommand)]
        deed: Cargo,
    },
    Chart {
        #[command(subcommand)]
        deed: Chart,
    },
    Npm {
        #[command(subcommand)]
        deed: Npm,
    },
    Oci {
        #[command(subcommand)]
        deed: Oci,
    },
    Cfworker {
        #[command(subcommand)]
        deed: Cfworker,
    },
    Site {
        #[command(subcommand)]
        deed: Site,
    },
}

#[derive(Subcommand)]
pub enum Cfworker {
    Publish,
    Rehearse,
}

#[derive(Subcommand)]
pub enum Cargo {
    Publish,
    Rehearse,
}

#[derive(Subcommand)]
pub enum Oci {
    Build,
    Publish,
}

#[derive(Subcommand)]
pub enum Chart {
    Package,
    Publish,
}

#[derive(Subcommand)]
pub enum Npm {
    Pack,
    Publish,
}

#[derive(Subcommand)]
pub enum Site {
    Deploy {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
    Inspect {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
    Plan {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
pub enum Binary {
    Activate,
    Assemble,
    Build,
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
    Inspect,
    Managers,
    Matrix,
    Publish,
    Smoke,
    Verify,
}

pub fn run(deed: Deed) -> i32 {
    let result = match deed {
        Deed::Binary { deed } => binary(deed),
        Deed::Cargo { deed } => cargo(deed),
        Deed::Chart { deed } => chart(deed),
        Deed::Npm { deed } => npm(deed),
        Deed::Oci { deed } => oci(deed),
        Deed::Cfworker { deed } => cfworker(deed),
        Deed::Site { deed } => site(deed),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb ship: {error}");
            1
        }
    }
}

fn cargo(deed: Cargo) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let attachment = engine::adaptor::registry::registry(&spec);
    match deed {
        Cargo::Publish => {
            sealed(&spec, release, version, spec.cargo.is_some())?;
            attachment.publish(version, &release.registry_token)
        }
        Cargo::Rehearse => attachment.rehearse(version, &release.registry_token),
    }
}

fn oci(deed: Oci) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let carrier = engine::adaptor::image::image(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        Oci::Build => carrier.build(version, &release.commit, &artifacts(release)?),
        Oci::Publish => {
            sealed(&spec, release, version, spec.oci.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

fn chart(deed: Chart) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = engine::adaptor::chart::chart(&spec);
    match deed {
        Chart::Package => carrier.package(version),
        Chart::Publish => {
            sealed(&spec, release, version, spec.chart.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

fn npm(deed: Npm) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = engine::adaptor::module::module(&spec);
    match deed {
        Npm::Pack => carrier.pack(version),
        Npm::Publish => {
            sealed(&spec, release, version, spec.npm.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

pub(super) fn registry_token(credential: &str) -> Result<&str, String> {
    let credential = required("PLUMB_RELEASE_REGISTRY_TOKEN", credential)?;
    credential
        .strip_prefix("Bearer ")
        .filter(|token| !token.chars().any(char::is_whitespace))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "PLUMB_RELEASE_REGISTRY_TOKEN must be a Cargo Bearer credential".into())
}

pub struct Identity<'a> {
    pub user: &'a str,
    pub token: &'a str,
}

fn sealed(
    spec: &super::release::model::Spec,
    release: &plumb::rig::Release,
    version: &str,
    declared: bool,
) -> Result<(), String> {
    if !spec.binary() {
        return Ok(());
    }
    let path = capsule(release)?;
    let (compiled, _) = super::release::record::Capsule::read(&path)?;
    if compiled.version != version {
        return Err(format!(
            "capsule seals {} while the projection carries {version}",
            compiled.version
        ));
    }
    if !declared {
        return Ok(());
    }
    verify::object(&compiled.seal)
}

fn cfworker(deed: Cfworker) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let seat = super::site::Worker {
        root: &rig.release.root,
        channel: required("PLUMB_RELEASE_CHANNEL", &rig.release.channel)?,
        version: required("PLUMB_RELEASE_VERSION", &rig.release.version)?,
    };
    super::site::worker(seat, matches!(deed, Cfworker::Publish))
}

fn site(deed: Site) -> Result<String, String> {
    match deed {
        Site::Deploy { root } => super::site::deploy(&root),
        Site::Inspect { root } => super::site::inspect(&root),
        Site::Plan { root } => super::site::plan(&root),
    }
}

fn binary(deed: Binary) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let manifest = rig.release.root.join("plumb.toml");
    let spec = super::release::model::Spec::read(&manifest)?;
    let release = &rig.release;
    match deed {
        Binary::Activate => storage::shift(&capsule(release)?, &rig.activate),
        Binary::Assemble => engine::package::product(&spec).assemble(
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &artifacts(release)?,
        ),
        Binary::Build => engine::package::product(&spec).build(engine::package::Build {
            target: required("PLUMB_RELEASE_TARGET", &release.target)?,
            version: required("PLUMB_RELEASE_VERSION", &release.version)?,
            channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
            artifacts: &artifacts(release)?,
        }),
        Binary::Dispatch { options } => super::operator::dispatch(options),
        Binary::Inspect => verify::binary(
            required("PLUMB_RELEASE_URL", &release.url)?,
            release.activated,
        ),
        Binary::Managers => manager::write(
            &manifest,
            required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &output(release)?,
        ),
        Binary::Matrix => engine::package::product(&spec).matrix(),
        Binary::Publish => storage::publish(&capsule(release)?, &rig.publish),
        Binary::Smoke => smoke::run(
            &manifest,
            required("PLUMB_RELEASE_URL", &release.url)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
        ),
        Binary::Verify => verify::run(&capsule(release)?, release.activated),
    }
}
