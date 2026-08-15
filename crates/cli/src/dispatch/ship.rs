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
    Site {
        #[command(subcommand)]
        deed: Site,
    },
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
    Assemble,
    Build,
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
    Managers,
    Matrix,
    Publish,
    Recovery {
        #[command(subcommand)]
        deed: super::operator::Recovery,
    },
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
    let attachment = engine::adaptor::registry::registry(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        Cargo::Publish => {
            sealed(&capsule(release)?, version)?;
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
        Oci::Build => carrier.build(version, &artifacts(release)?),
        Oci::Publish => {
            sealed(&capsule(release)?, version)?;
            carrier.publish(version, &registry(release)?)
        }
    }
}

fn chart(deed: Chart) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let carrier = engine::adaptor::chart::chart(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        Chart::Package => carrier.package(version),
        Chart::Publish => {
            sealed(&capsule(release)?, version)?;
            carrier.publish(version, &registry(release)?)
        }
    }
}

fn npm(deed: Npm) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = super::release::model::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let carrier = engine::adaptor::module::module(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        Npm::Pack => carrier.pack(version),
        Npm::Publish => {
            sealed(&capsule(release)?, version)?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

fn registry(release: &plumb::rig::Release) -> Result<Identity<'_>, String> {
    Ok(Identity {
        user: required("PLUMB_RELEASE_REGISTRY_ACCOUNT", &release.registry_account)?,
        token: required("PLUMB_RELEASE_REGISTRY_TOKEN", &release.registry_token)?,
    })
}

pub struct Identity<'a> {
    pub user: &'a str,
    pub token: &'a str,
}

fn sealed(path: &std::path::Path, version: &str) -> Result<(), String> {
    let (compiled, _) = super::release::record::Capsule::read(path)?;
    if compiled.version != version {
        return Err(format!(
            "capsule seals {} while the projection carries {version}",
            compiled.version
        ));
    }
    Ok(())
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
        Binary::Managers => manager::write(
            &manifest,
            required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &output(release)?,
        ),
        Binary::Matrix => engine::package::product(&spec).matrix(),
        Binary::Publish => storage::publish(&capsule(release)?, &rig.publish),
        Binary::Recovery { deed } => super::operator::recovery(deed, &spec),
        Binary::Smoke => smoke::run(
            &manifest,
            required("PLUMB_RELEASE_URL", &release.url)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
        ),
        Binary::Verify => verify::run(&capsule(release)?, release.activated),
    }
}
