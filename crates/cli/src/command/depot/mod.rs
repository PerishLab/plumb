pub mod notes;
mod seat;
mod store;

use clap::Subcommand;
use plumb::rig::Rig;
use plumb::snapshot::{Refusal, Snapshot};
use std::path::{Path, PathBuf};
use std::process::Output;

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
            objects: seat.objects().to_vec(),
        })),
    }
}

pub fn occupied(root: &Path, version: &str) -> Result<Option<Vec<record::Object>>, String> {
    let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
    let depot = spec.derivative(plumb::depot::v2::Kind::Changelog)?;
    let channel = crate::command::release::channel(version)?;
    if let Some(manifest) = seat::derivative(seat::Query {
        source: &depot.source,
        product: &spec.product,
        channel: &channel,
        version,
        derivative: plumb::depot::v2::Kind::Changelog,
    })? {
        return Ok(Some(manifest.objects));
    }
    seat::notes(&depot.source, version).map(|held| held.map(|notes| notes.objects))
}

pub fn carried(path: &str, factory: &'static str) -> String {
    held()
        .read(path, factory)
        .unwrap_or_else(|_| factory.to_string())
}

pub fn held() -> Held {
    seat::held(&Rig::resolve(None).unwrap_or_default().rules.seat)
}

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Publish a Release-bound configuration snapshot after exact validation")]
    Publish {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Publish the changelog derivative one stable Release owes")]
    Changelog {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        from: String,
        #[arg(long)]
        keep: bool,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Bring the local Plumb rules seat to the version its channel names")]
    Sync,
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
    let over = PathBuf::from(&rig.rules.seat);
    match deed {
        Deed::Publish { root, version, dry } => Tree(&PathBuf::from(root)).publish(&version, dry),
        Deed::Changelog {
            root,
            version,
            from,
            keep,
            dry,
        } => Tree(&PathBuf::from(root)).notes(Wanted {
            version: &version,
            from: &from,
            keep,
            dry,
        }),
        Deed::Sync => seat::sync(&rig.rules.source, &rig.rules.channel, &over),
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

struct Wanted<'a> {
    version: &'a str,
    from: &'a str,
    keep: bool,
    dry: bool,
}

struct Tree<'a>(&'a Path);

impl Tree<'_> {
    fn publish(&self, version: &str, dry: bool) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let spec = crate::shape::release::Spec::read(&self.0.join("plumb.toml"))?;
        let depot = spec.derivative(plumb::depot::v2::Kind::Configuration)?;
        let release = crate::command::release::depot(&spec);
        let binding = release.binding(version, true)?;
        let snapshot = Snapshot::read(self.0).map_err(|error| error.to_string())?;
        self.clean()?;
        let plan = record::Batch::configuration(
            &snapshot,
            record::Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: super::clock::mark()?,
                commit: self.commit()?,
            },
        )?;
        crate::command::release::validate_depot(&spec, &binding, &plan)?;
        if dry {
            return plan.manifest.encode();
        }
        let advance = release.current(&binding.release)?;
        rig.depot.authority.load()?;
        store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)
    }

    fn notes(&self, wanted: Wanted<'_>) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let staged = wanted.from.is_empty();
        let source = if staged {
            plumb::seat::tmp(self.0, seat::KEY)
                .join("changelog")
                .join(wanted.version)
        } else {
            PathBuf::from(wanted.from)
        };
        let proof = crate::command::changelog::prove(self.0, &source, wanted.version)?;
        let spec = crate::shape::release::Spec::read(&self.0.join("plumb.toml"))?;
        let depot = spec.derivative(plumb::depot::v2::Kind::Changelog)?;
        let release = crate::command::release::depot(&spec);
        let binding = release.binding(wanted.version, false)?;
        if binding.release.channel != "stable" {
            return Err(format!(
                "the {} channel does not owe a changelog derivative",
                binding.release.channel
            ));
        }
        if binding.release.commit != proof.candidate {
            return Err(format!(
                "changelog proves commit {}, but release {} seals {}",
                proof.candidate, binding.release.version, binding.release.commit
            ));
        }
        let batch = notes::Batch::gather(&source)?;
        let plan = record::Batch::changelog(
            record::Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: super::clock::mark()?,
                commit: proof.candidate.clone(),
            },
            batch.bodies,
        )?;
        if wanted.dry {
            return Ok(format!(
                "{}\n{} lines within a budget of {} for {} units",
                plan.manifest.encode()?,
                proof
                    .languages
                    .values()
                    .map(|held| held.lines)
                    .max()
                    .unwrap_or_default(),
                proof
                    .languages
                    .values()
                    .map(|held| held.budget)
                    .max()
                    .unwrap_or_default(),
                proof.units
            ));
        }
        let advance = release.current(&binding.release)?;
        rig.depot.authority.load()?;
        let held = store::Remote::new(&rig.depot.authority)?.derive(&plan, advance)?;
        if staged && !wanted.keep {
            std::fs::remove_dir_all(&source)
                .map_err(|error| format!("cannot clear {}: {error}", source.display()))?;
            return Ok(format!("{held}, and cleared {}", source.display()));
        }
        Ok(held)
    }

    fn clean(&self) -> Result<(), String> {
        let mut args = vec!["status", "--porcelain", "--"];
        let roots = record::configuration(self.0)?;
        for (root, _) in &roots {
            args.push(root);
        }
        let output = self.git(&args)?;
        if !output.status.success() {
            return Err(format!(
                "cannot read the working tree: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let dirty = String::from_utf8_lossy(&output.stdout);
        if dirty.trim().is_empty() {
            Ok(())
        } else {
            Err(format!(
                "depot roots carry uncommitted change:\n{}",
                dirty.trim()
            ))
        }
    }

    fn commit(&self) -> Result<String, String> {
        let output = self.git(&["rev-parse", "HEAD"])?;
        if !output.status.success() {
            return Err("cannot read HEAD".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn git(&self, args: &[&str]) -> Result<Output, String> {
        std::process::Command::new("git")
            .current_dir(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}
