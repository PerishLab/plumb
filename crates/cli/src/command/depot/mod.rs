mod seat;

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
    let spec = crate::shape::release::Spec::controller(root)?;
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
