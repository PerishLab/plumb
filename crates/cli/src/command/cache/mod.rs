mod bucket;
mod graph;
mod sweep;

pub(crate) use bucket::{adopt, home, lease, seat};
pub(crate) use sweep::{now, settle};

use clap::Subcommand;
use serde::Serialize;
use std::path::Path;

const DAY: u64 = 24 * 60 * 60;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Report every Guard build cache with its status, size and idle targets")]
    Status {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Preview, or with --apply remove, idle Guard build caches and units")]
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
    applied: bool,
    buckets: Vec<sweep::Entry>,
    reclaimable: u64,
    reclaimed: u64,
}

pub fn run(deed: Deed) -> i32 {
    let (apply, json) = match deed {
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
    let now = sweep::now();
    let buckets: Vec<sweep::Entry> = graph::Node(&bucket::root(&home))
        .listing()?
        .iter()
        .filter(|path| path.is_dir())
        .map(|path| sweep::survey(&home, path, now, apply))
        .collect();
    Ok(Report {
        schema: bucket::SCHEMA,
        idle: sweep::IDLE,
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
            "{:<12} {:<9} {:>10}  reclaimable {:>10} in {:>4} targets  {}",
            entry.bucket.chars().take(12).collect::<String>(),
            status.as_str().unwrap_or_default(),
            bytes(entry.bytes),
            bytes(entry.reclaimable),
            entry.targets.len(),
            entry.identity.as_deref().unwrap_or("-"),
        );
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
        "{verb}: {} of {} reclaimable; idle after {} days",
        bytes(report.reclaimed),
        bytes(report.reclaimable),
        report.idle / DAY
    );
}

fn bytes(count: u64) -> String {
    format!("{:.1} MiB", count as f64 / (1024.0 * 1024.0))
}

pub(crate) fn tour(own: &Path) -> Result<(), String> {
    let home = bucket::home()?;
    let stamp = home.join("state").join("guard").join("cargo").join("tour");
    let now = sweep::now();
    if now.saturating_sub(graph::Node(&stamp).stamp()) < DAY {
        return Ok(());
    }
    if let Some(parent) = stamp.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&stamp, now.to_string()).map_err(|error| error.to_string())?;
    for path in graph::Node(&bucket::root(&home)).listing()? {
        if path.is_dir() && path != own {
            sweep::survey(&home, &path, now, true);
        }
    }
    Ok(())
}
