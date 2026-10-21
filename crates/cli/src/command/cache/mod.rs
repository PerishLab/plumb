mod bucket;
mod disk;
mod graph;
mod parcel;
mod plan;
mod pressure;
mod sweep;
mod transfer;
pub(crate) mod transport;

pub(crate) use bucket::{adopt, home, lease, seat};
pub(crate) use sweep::{now, settle};

use clap::Subcommand;
use disk::Disk;
use serde::Serialize;
use std::path::{Path, PathBuf};

const DAY: u64 = 24 * 60 * 60;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Declare owned Guard compilation transport", long_about = plumb::seat::resource!("help/cache.txt"))]
    Transport(transport::Input),
    #[command(
        about = "Report every Guard build cache with its status, size and reclaimable targets"
    )]
    Status {
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Preview, or with --apply remove, idle, superseded and unmarked Guard build caches"
    )]
    Reclaim {
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    idle: u64,
    span: u64,
    disk: Disk,
    applied: bool,
    buckets: Vec<sweep::Entry>,
    reclaimable: u64,
    reclaimed: u64,
}

pub fn run(deed: Deed) -> i32 {
    let (apply, json) = match deed {
        Deed::Transport(input) => return transport::run(input),
        Deed::Status { json } => (false, json),
        Deed::Reclaim { apply, json } => (apply, json),
    };
    let report = match survey(apply) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("plumb cache: {error}");
            return 1;
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("cache report should encode")
        );
    } else {
        render(&report);
    }
    i32::from(
        report.buckets.iter().any(|entry| {
            entry.refusal.is_some() && !matches!(entry.status, sweep::Status::Unknown)
        }),
    )
}

fn survey(apply: bool) -> Result<Report, String> {
    let home = bucket::home()?;
    let root = bucket::root(&home);
    let now = sweep::now();
    let disk = Disk::read(&home, &root)?;
    let sweep = sweep::Sweep {
        apply,
        span: disk.span(),
        operator: true,
    };
    let mut held: Vec<(PathBuf, sweep::Entry)> = graph::Node(&root)
        .listing()?
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|path| {
            let entry = sweep::survey(&home, &path, now, sweep);
            (path, entry)
        })
        .collect();
    let keep = pressure::recent(&held);
    let relief = pressure::Relief {
        home: &home,
        root: &root,
        apply,
        keep: keep.as_deref(),
    };
    relief.run(disk, &mut held)?;
    let buckets: Vec<sweep::Entry> = held.into_iter().map(|(_, entry)| entry).collect();
    Ok(Report {
        schema: bucket::SCHEMA,
        idle: sweep::IDLE,
        span: sweep.span,
        disk,
        applied: apply,
        reclaimable: buckets.iter().map(|entry| entry.reclaimable).sum(),
        reclaimed: buckets.iter().map(|entry| entry.reclaimed).sum(),
        buckets,
    })
}

fn render(report: &Report) {
    for entry in &report.buckets {
        let status = serde_json::to_value(entry.status).unwrap_or_default();
        println!(
            "{:<12} {:<9} {:>10}  working {:>10}  superseded {:>10}  reclaimable {:>10} in {:>4} targets  {}",
            entry.bucket.chars().take(12).collect::<String>(),
            status.as_str().unwrap_or_default(),
            bytes(entry.bytes),
            bytes(entry.working),
            bytes(entry.superseded),
            bytes(entry.reclaimable),
            entry.targets.len(),
            entry.identity.as_deref().unwrap_or("-"),
        );
        if entry.evicted {
            println!("             pressure: whole bucket, least recently used first");
        }
        if let Some(refusal) = &entry.refusal {
            println!("             refused: {refusal}");
        }
    }
    let verb = if report.applied {
        "reclaimed"
    } else {
        "preview"
    };
    println!();
    println!(
        "{verb}: {} of {} reclaimable; idle after {} days; a live bucket keeps {}x its working set",
        bytes(report.reclaimed),
        bytes(report.reclaimable),
        report.idle / DAY,
        report.span
    );
    println!(
        "disk: {} of {} available; pressure below {}% ({}): {}",
        bytes(report.disk.available),
        bytes(report.disk.total),
        report.disk.reserve,
        bytes(report.disk.line),
        if report.disk.pressure { "yes" } else { "no" }
    );
    if report.disk.pressure {
        let short = bytes(report.disk.deficit);
        if report.reclaimable >= report.disk.deficit {
            println!(
                "pressure: {short} below the line; Guard caches hold it, and reclaiming these targets restores the line"
            );
        } else {
            println!(
                "pressure: {short} below the line; Guard caches can free {} of it, the rest is outside them",
                bytes(report.reclaimable)
            );
        }
    }
}

fn bytes(count: u64) -> String {
    format!("{:.1} MiB", count as f64 / (1024.0 * 1024.0))
}

pub(crate) fn tour(own: &Path) -> Result<(), String> {
    let home = bucket::home()?;
    let root = bucket::root(&home);
    let stamp = home.join("state").join("guard").join("cargo").join("tour");
    let now = sweep::now();
    let disk = Disk::read(&home, &root)?;
    if !disk.pressure && now.saturating_sub(graph::Node(&stamp).stamp()) < DAY {
        return Ok(());
    }
    if let Some(parent) = stamp.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&stamp, now.to_string()).map_err(|error| error.to_string())?;
    let sweep = sweep::Sweep {
        apply: true,
        span: disk.span(),
        operator: false,
    };
    let mut held: Vec<(PathBuf, sweep::Entry)> = graph::Node(&root)
        .listing()?
        .into_iter()
        .filter(|path| path.is_dir() && path != own)
        .map(|path| {
            let entry = sweep::survey(&home, &path, now, sweep);
            (path, entry)
        })
        .collect();
    let relief = pressure::Relief {
        home: &home,
        root: &root,
        apply: true,
        keep: None,
    };
    relief.run(disk, &mut held)
}
