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
    let Deed::Binary { deed } = deed;
    match binary(deed) {
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
