pub mod notes;
pub mod record;
mod seat;
mod store;

use clap::Subcommand;
use plumb::rig::Rig;
use plumb::snapshot::Snapshot;
use std::path::{Path, PathBuf};
use std::process::Output;

pub use seat::Held;

pub fn manifest() -> Result<Option<record::Manifest>, String> {
    match held() {
        Held::Factory => Ok(None),
        Held::Blind(error) => Err(error),
        Held::Seat(base) => {
            let file = base.join(record::LEAF);
            let text = std::fs::read_to_string(&file)
                .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
            record::Manifest::parse(&text).map(Some)
        }
    }
}

pub fn occupied(version: &str) -> Result<Option<notes::Notes>, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    seat::notes(&rig.depot.source, version)
}

pub fn held() -> Held {
    seat::held(&Rig::resolve(None).unwrap_or_default().depot.seat)
}

#[derive(Subcommand)]
pub enum Deed {
    Publish {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long, default_value = "")]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
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
    Sync,
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
    let over = PathBuf::from(&rig.depot.seat);
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
        Deed::Sync => seat::sync(&rig.depot.source, &rig.depot.channel, &over),
        Deed::Show => show(&rig, &over),
    }
}

fn show(rig: &Rig, over: &Path) -> Result<String, String> {
    let base = seat::root(over)?;
    let held = seat::held(over);
    let mark = match &held {
        Held::Factory => "factory".to_string(),
        Held::Blind(error) => return Err(error.clone()),
        Held::Seat(_) => held.mark().unwrap_or_default(),
    };
    Ok(format!(
        "depot {} {}\n  source {}\n  seat   {}",
        rig.depot.channel,
        mark,
        rig.depot.source,
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
        let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let snapshot = Snapshot::read(self.0).map_err(|error| error.to_string())?;
        self.clean()?;
        let metadata = record::Metadata {
            version: super::clock::mark()?,
            source: rig.depot.source.trim_end_matches('/').to_string(),
            channel: rig.depot.channel.clone(),
            commit: self.commit()?,
        };
        let schema = record::Schema {
            format: record::FORMAT,
            version: if version.is_empty() {
                plumb::version!("PLUMB").to_string()
            } else {
                version.to_string()
            },
        };
        let plan = record::Plan::gather(&snapshot, metadata, schema)?;
        if dry {
            return plan.manifest.encode();
        }
        store::Remote::new(&rig.depot.authority)?.publish(&plan)
    }

    fn notes(&self, wanted: Wanted<'_>) -> Result<String, String> {
        let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let staged = wanted.from.is_empty();
        let source = if staged {
            plumb::seat::tmp(self.0, seat::KEY)
                .join("changelog")
                .join(wanted.version)
        } else {
            PathBuf::from(wanted.from)
        };
        let commit = self.commit()?;
        let proof = crate::shape::changelog::prove(self.0, &source, wanted.version, &commit)?;
        let batch = notes::Batch::gather(&source, wanted.version, &commit)?;
        if wanted.dry {
            return Ok(format!(
                "{}\n{} lines within a budget of {} for {} units",
                batch.notes.encode()?,
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
        let held = store::Remote::new(&rig.depot.authority)?.stock(&batch)?;
        if staged && !wanted.keep {
            std::fs::remove_dir_all(&source)
                .map_err(|error| format!("cannot clear {}: {error}", source.display()))?;
            return Ok(format!("{held}, and cleared {}", source.display()));
        }
        Ok(held)
    }

    fn clean(&self) -> Result<(), String> {
        let mut args = vec!["status", "--porcelain", "--"];
        for (root, _) in record::ROOTS {
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
