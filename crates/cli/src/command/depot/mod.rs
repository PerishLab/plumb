mod configuration;
mod knowledge;
mod product;
mod projection;
mod seat;
mod store;

use clap::Subcommand;
use plumb::rig::Rig;
use plumb::snapshot::{Refusal, Snapshot};
use std::path::{Path, PathBuf};

use crate::shape::depot::{self as record, Evidence};

pub use seat::Held;

pub fn observe(snapshot: &Result<Snapshot, Refusal>) -> Evidence {
    let manifest = match manifest() {
        Ok(None) => return Evidence::Absent,
        Ok(Some(manifest)) => manifest,
        Err(error) => return Evidence::Blind(error),
    };
    let inventory = snapshot.as_ref().ok().and_then(|snapshot| {
        record::configuration(snapshot.root())
            .ok()
            .map(|_| record::inventory(snapshot).map(|(objects, _)| objects))
    });
    Evidence::Held {
        manifest,
        inventory,
    }
}

pub fn manifest() -> Result<Option<record::Manifest>, String> {
    match held() {
        Held::Absent => Ok(None),
        Held::Blind(error) => Err(error),
        Held::Seat(seat) => Ok(Some(record::Manifest {
            mark: seat.mark().to_string(),
            floor: seat.floor().to_string(),
            version: seat.version().map(str::to_string),
            objects: seat.objects().to_vec(),
        })),
    }
}

pub fn changelog(
    root: &Path,
    version: &str,
) -> Result<Option<plumb::depot::v3::Generation>, String> {
    let spec = crate::shape::release::Spec::resolve(root)?;
    let depot = spec.route(plumb::depot::v3::Kind::Changelog)?;
    let channel = crate::command::release::channel(version)?;
    plumb::depot::v3::Generation::latest(plumb::depot::v3::Query {
        source: depot,
        product: &spec.product,
        channel: &channel,
        version,
        kind: plumb::depot::v3::Kind::Changelog,
    })
}

pub fn carried(path: &str, factory: &'static str) -> String {
    held()
        .read(path, factory)
        .unwrap_or_else(|_| factory.to_string())
}

pub fn held() -> Held {
    seat::held(Path::new(""))
}

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Publish a configuration generation after marker-exact validation")]
    Configuration {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        marker: String,
        #[arg(long)]
        from: String,
        #[arg(
            long,
            help = "Use one exact local replacement for the selected released validator"
        )]
        recovery_validator: Option<PathBuf>,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Move one channel pointer onto the immutable release named by a marker")]
    Channel {
        #[arg(long)]
        marker: String,
    },
    #[command(about = "Move the stable manager roots onto the immutable release named by a marker")]
    Managers {
        #[arg(long)]
        marker: String,
    },
    #[command(about = "Deploy one immutable worker version bound to a release marker")]
    #[command(hide = true)]
    Worker {
        #[arg(long)]
        marker: String,
        #[arg(long)]
        request: String,
    },
    #[command(about = "Publish the changelog generation bound to a stable Release marker")]
    Changelog {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        marker: String,
        #[arg(long)]
        from: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Publish the skill generation one product version carries")]
    Skill {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        marker: String,
        #[arg(long)]
        from: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Report the Plumb rules source, local seat, and held version")]
    Show,
}

pub fn run(deed: Deed) -> i32 {
    match execute(deed) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb depot: {error}");
            1
        }
    }
}

fn execute(deed: Deed) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let over = PathBuf::new();
    match deed {
        Deed::Configuration {
            root,
            marker,
            from,
            recovery_validator,
            dry,
        } => configuration::Tree(&PathBuf::from(root)).publish(
            &marker,
            &from,
            recovery_validator.as_deref(),
            dry,
        ),
        Deed::Channel { marker } => projection::project(&marker, projection::Kind::Channel),
        Deed::Managers { marker } => projection::project(&marker, projection::Kind::Managers),
        Deed::Worker { marker, request } => projection::worker(&marker, &request),
        Deed::Changelog {
            root,
            marker,
            from,
            dry,
        } => knowledge::changelog(
            &PathBuf::from(root),
            knowledge::Wanted {
                marker: &marker,
                from: &from,
                dry,
            },
        ),
        Deed::Skill {
            root,
            marker,
            from,
            dry,
        } => knowledge::skill(
            &PathBuf::from(root),
            knowledge::Wanted {
                marker: &marker,
                from: &from,
                dry,
            },
        ),
        Deed::Show => show(&rig, &over),
    }
}

fn show(rig: &Rig, over: &Path) -> Result<String, String> {
    let base = seat::root(over)?;
    let held = seat::held(over);
    let mark = match &held {
        Held::Absent => "unsynced".to_string(),
        Held::Blind(error) => return Err(error.clone()),
        Held::Seat(_) => held.mark().unwrap_or_default(),
    };
    Ok(format!(
        "depot {} {}\n  source {}\n  seat   {}",
        rig.rules.channel,
        mark,
        rig.rules.source,
        base.display()
    ))
}
