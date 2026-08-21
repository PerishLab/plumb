mod adaptor;
mod archive;
mod attachment;
mod package;
mod site;
mod skill;
mod smoke;

use super::release::{artifacts, capsule, manager, output, required, storage, verify};
use clap::Subcommand;
use plumb::rig::Rig;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Build, prove, and publish the product's own artifacts")]
    Binary {
        #[command(subcommand)]
        deed: Binary,
    },
    #[command(about = "Carry the release to its Cargo registry")]
    Cargo {
        #[command(subcommand)]
        deed: Cargo,
    },
    #[command(about = "Carry the release to its Helm chart registry")]
    Chart {
        #[command(subcommand)]
        deed: Chart,
    },
    #[command(about = "Carry the release to its npm registry")]
    Npm {
        #[command(subcommand)]
        deed: Npm,
    },
    #[command(about = "Carry the release to its image registry")]
    Oci {
        #[command(subcommand)]
        deed: Oci,
    },
    #[command(about = "Put the site's version worker on its edge")]
    Cfworker {
        #[command(subcommand)]
        deed: Cfworker,
    },
    #[command(
        about = "Project a declared site onto its edge",
        long_about = super::depot::carried("help/ship/site.txt", plumb::seat::resource!("help/ship/site.txt"))
    )]
    Site {
        #[command(subcommand)]
        deed: Site,
    },
}

#[derive(Subcommand)]
pub enum Cfworker {
    #[command(about = "Build the version worker and put it on its edge")]
    Publish,
    #[command(about = "Prove the previews the worker needs are reachable, and build nothing")]
    Rehearse,
}

#[derive(Subcommand)]
pub enum Cargo {
    #[command(about = "Publish every declared crate, in the order the attachment declares")]
    Publish,
    #[command(about = "Package every declared crate without uploading anything")]
    Rehearse,
}

#[derive(Subcommand)]
pub enum Oci {
    #[command(
        about = "Build the declared image from the Containerfile with this release's payload"
    )]
    Build,
    #[command(about = "Push the built image, or accept the identical one already published")]
    Publish,
}

#[derive(Subcommand)]
pub enum Chart {
    #[command(about = "Stamp the chart with this version and package it")]
    Package,
    #[command(about = "Package the chart and push it to its registry")]
    Publish,
}

#[derive(Subcommand)]
pub enum Npm {
    #[command(about = "Stamp and pack every declared package")]
    Pack,
    #[command(about = "Pack every declared package and publish it")]
    Publish,
}

#[derive(Subcommand)]
pub enum Site {
    #[command(
        about = "Build, upload, and prove the edge serves what was built",
        long_about = super::depot::carried("help/ship/site/deploy.txt", plumb::seat::resource!("help/ship/site/deploy.txt"))
    )]
    Deploy {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
    #[command(about = "Read what Cloudflare currently holds for this site")]
    Inspect {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
    #[command(about = "Derive the site from the build without reaching any credential")]
    Plan {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
pub enum Binary {
    #[command(about = "Shift the stable manager roots onto this published version")]
    Activate,
    #[command(about = "Gather the declared assets, and the skill archive, into the artifact seat")]
    Assemble,
    #[command(about = "Compile the product for one target triple and archive it")]
    Build,
    #[command(about = "Dispatch the release workflow for one version on the forge")]
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
    #[command(about = "Read what the release surface publishes for this version")]
    Inspect,
    #[command(about = "Render the manager scripts this channel and version owe")]
    Managers,
    #[command(about = "Print the declared target matrix as one build JSON")]
    Matrix,
    #[command(about = "Upload every capsule object and its seal, then prove they are reachable")]
    Publish,
    #[command(about = "Install this version through its published manager, then clean up")]
    Smoke,
    #[command(about = "Prove the capsule's objects are published, and its projection if activated")]
    Verify,
}

pub fn run(deed: Deed) -> i32 {
    let result = match deed {
        Deed::Binary { deed } => binary(deed),
        Deed::Cargo { deed } => attachment::cargo(deed),
        Deed::Chart { deed } => attachment::chart(deed),
        Deed::Npm { deed } => attachment::npm(deed),
        Deed::Oci { deed } => attachment::oci(deed),
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

fn cfworker(deed: Cfworker) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let seat = site::Worker {
        root: &rig.release.root,
        channel: required("PLUMB_RELEASE_CHANNEL", &rig.release.channel)?,
        version: required("PLUMB_RELEASE_VERSION", &rig.release.version)?,
    };
    site::worker(seat, matches!(deed, Cfworker::Publish))
}

fn site(deed: Site) -> Result<String, String> {
    match deed {
        Site::Deploy { root } => site::deploy(&root),
        Site::Inspect { root } => site::inspect(&root),
        Site::Plan { root } => site::plan(&root),
    }
}

fn binary(deed: Binary) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let manifest = rig.release.root.join("plumb.toml");
    let spec = super::release::model::Spec::read(&manifest)?;
    let release = &rig.release;
    match deed {
        Binary::Activate => storage::shift(&capsule(release)?, &rig.activate),
        Binary::Assemble => package::product(&spec).assemble(
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &artifacts(release)?,
        ),
        Binary::Build => package::product(&spec).build(package::Build {
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
        Binary::Matrix => package::product(&spec).matrix(),
        Binary::Publish => storage::publish(&capsule(release)?, &rig.publish),
        Binary::Smoke => smoke::run(
            &manifest,
            required("PLUMB_RELEASE_URL", &release.url)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
        ),
        Binary::Verify => verify::run(&capsule(release)?, release.activated),
    }
}
