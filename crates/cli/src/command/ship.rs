pub(in crate::command) mod adaptor;
mod archive;
mod attachment;
mod package;
pub(in crate::command) mod site;
mod skill;
mod smoke;
mod transport;

use super::release::{artifacts, capsule, manager, output, required, storage, verify};
use clap::Subcommand;
use plumb::rig::Rig;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Dispatch every declared distribution medium for one release marker")]
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
    #[command(about = "Execute one closed request emitted by the ship plan")]
    #[command(hide = true)]
    Execute {
        #[arg(long)]
        request: String,
    },
    #[command(about = "Resolve one marker into the exact ship execution graph")]
    #[command(hide = true)]
    Resolve {
        #[arg(long)]
        marker: String,
        #[arg(long)]
        atom: String,
    },
    #[command(about = "Compile the immutable capsule carried by this ship run")]
    #[command(hide = true)]
    Compile,
    #[command(about = "Derive and prove this marker's ship plan as JSON")]
    #[command(hide = true)]
    Plan,
    #[command(about = "Fetch the exact immutable release a stable ship promotes")]
    #[command(hide = true)]
    Promote,
    #[command(about = "Report the immutable and mutable media this ship transaction carries")]
    #[command(hide = true)]
    Surface,
    #[command(about = "Read a shipped release back and verify it against its seal")]
    #[command(hide = true)]
    Inspect,
    #[command(about = "Build, prove, and publish the product's own artifacts")]
    #[command(hide = true)]
    Binary {
        #[command(subcommand)]
        deed: Binary,
    },
    #[command(about = "Carry the release to its Cargo registry")]
    #[command(hide = true)]
    Cargo {
        #[command(subcommand)]
        deed: Cargo,
    },
    #[command(about = "Carry the release to its Helm chart registry")]
    #[command(hide = true)]
    Chart {
        #[command(subcommand)]
        deed: Chart,
    },
    #[command(about = "Carry the release to its npm registry")]
    #[command(hide = true)]
    Npm {
        #[command(subcommand)]
        deed: Npm,
    },
    #[command(about = "Carry the release to its image registry")]
    #[command(hide = true)]
    Oci {
        #[command(subcommand)]
        deed: Oci,
    },
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
    #[command(about = "Publish the exact image from source or a reusable workload")]
    Exact {
        #[arg(long, default_value = "{\"type\":\"none\",\"source\":\"\"}")]
        reuse: String,
    },
    #[command(
        about = "Build the declared image from the Containerfile with this release's payload"
    )]
    Build,
    #[command(about = "Push the built image, or accept the identical one already published")]
    Publish,
}

#[derive(Subcommand)]
pub enum Chart {
    #[command(about = "Publish the exact chart from source or a reusable workload")]
    Exact {
        #[arg(long, default_value = "{\"type\":\"none\",\"source\":\"\"}")]
        reuse: String,
    },
    #[command(about = "Stamp the chart with this version and package it")]
    Package,
    #[command(about = "Package the chart and push it to its registry")]
    Publish,
}

#[derive(Subcommand)]
pub enum Npm {
    #[command(about = "Publish one exact package from source or a reusable workload")]
    Exact {
        #[arg(long)]
        package: String,
        #[arg(long, default_value = "{\"type\":\"none\",\"source\":\"\"}")]
        reuse: String,
    },
    #[command(about = "Stamp and pack every declared package")]
    Pack,
    #[command(about = "Pack every declared package and publish it")]
    Publish,
}

#[derive(Subcommand)]
pub enum Binary {
    #[command(about = "Gather the declared assets, and the skill archive, into the artifact seat")]
    Assemble,
    #[command(about = "Compile the product for one target triple and archive it")]
    Build,
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
        Deed::Dispatch { options } => super::operator::dispatch(options),
        Deed::Execute { request } => transport::execute(&request),
        Deed::Resolve { marker, atom } => transport::resolve(&marker, &atom),
        Deed::Compile => carry(Carry::Compile),
        Deed::Plan => carry(Carry::Plan),
        Deed::Promote => carry(Carry::Promote),
        Deed::Surface => carry(Carry::Surface),
        Deed::Inspect => carry(Carry::Inspect),
        Deed::Binary { deed } => binary(deed),
        Deed::Cargo { deed } => attachment::cargo(deed),
        Deed::Chart { deed } => attachment::chart(deed),
        Deed::Npm { deed } => attachment::npm(deed),
        Deed::Oci { deed } => attachment::oci(deed),
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

enum Carry {
    Compile,
    Inspect,
    Plan,
    Promote,
    Surface,
}

fn carry(deed: Carry) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    spec.ship()?;
    match deed {
        Carry::Compile => super::release::Product::new(&spec).compile(&rig.release),
        Carry::Inspect => verify::inspect(
            required("PLUMB_RELEASE_URL", &rig.release.url)?,
            rig.release.activated,
        ),
        Carry::Plan => super::release::plan::plan(
            &spec,
            required("PLUMB_RELEASE_SOURCE", &rig.release.source)?,
            required("PLUMB_RELEASE_COMMIT", &rig.release.commit)?,
        ),
        Carry::Promote => super::release::Product::new(&spec).promote(&rig.release),
        Carry::Surface => super::release::plan::surface(&spec),
    }
}

fn binary(deed: Binary) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    spec.ship()?;
    let release = &rig.release;
    match deed {
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
        Binary::Inspect => verify::binary(
            required("PLUMB_RELEASE_URL", &release.url)?,
            release.activated,
        ),
        Binary::Managers => manager::write(
            &spec,
            required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
            &output(release)?,
        ),
        Binary::Matrix => package::product(&spec).matrix(),
        Binary::Publish => storage::publish(&capsule(release)?, &rig.publish),
        Binary::Smoke => smoke::run(
            &spec,
            required("PLUMB_RELEASE_URL", &release.url)?,
            required("PLUMB_RELEASE_VERSION", &release.version)?,
        ),
        Binary::Verify => verify::run(&capsule(release)?, release.activated),
    }
}
