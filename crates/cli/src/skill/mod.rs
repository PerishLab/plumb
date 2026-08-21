use clap::Subcommand;
use plumb::rig::Rig;
use plumb::skill::{Action, Ask, Done, Kit, Report};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Install a released brief into every agent seat this machine carries")]
    Install {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Move installed briefs to the version the channel names")]
    Upgrade {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Report which brief each agent seat holds and against which binary")]
    Status {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Unpack a brief into a staging path without touching an agent seat")]
    Stage {
        #[arg(long)]
        channel: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        path: PathBuf,
    },
    #[command(about = "List the agent seats this machine carries")]
    List,
    #[command(about = "Remove an installed brief from the seats that own it")]
    Uninstall,
}

pub fn run(deed: Deed) -> i32 {
    let rig = match Rig::resolve(None) {
        Ok(rig) => rig,
        Err(error) => return sour(&error.to_string()),
    };
    if rig.home.is_empty() {
        return sour("no data home; set PLUMB_HOME");
    }
    let kit = Kit {
        name: "plumb".to_string(),
        home: plumb::config::home().unwrap_or_else(|| PathBuf::from(".")),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases.clone(),
    };
    act(&kit, deed)
}

fn act(kit: &Kit, deed: Deed) -> i32 {
    match deed {
        Deed::Install {
            channel,
            version,
            path,
            force,
        } => told(
            "installed",
            kit.install(&Ask {
                channel,
                version,
                path,
                force,
            }),
            false,
        ),
        Deed::Upgrade {
            channel,
            version,
            dry_run,
            json,
        } => {
            let ask = Ask {
                channel,
                version,
                ..Ask::default()
            };
            if dry_run {
                report("upgrade_dry_run", kit.status(&ask), json)
            } else {
                told("upgraded", kit.upgrade(&ask), json)
            }
        }
        Deed::Status {
            channel,
            version,
            json,
        } => report(
            "status",
            kit.status(&Ask {
                channel,
                version,
                ..Ask::default()
            }),
            json,
        ),
        Deed::Stage {
            channel,
            version,
            path,
        } => told(
            "staged",
            kit.stage(&Ask {
                channel,
                version: Some(version),
                path: Some(path),
                ..Ask::default()
            }),
            false,
        ),
        Deed::List => tell(kit),
        Deed::Uninstall => told("removed", kit.uninstall(), false),
    }
}

fn tell(kit: &Kit) -> i32 {
    match kit.list() {
        Ok(records) => {
            for record in &records {
                println!(
                    "  {} {} {}",
                    record.agent,
                    record.version,
                    record.path.display()
                );
            }
            if records.is_empty() {
                println!("  no managed skill");
            }
            0
        }
        Err(error) => sour(&error.to_string()),
    }
}

fn told(action: &str, held: Result<Done, plumb::skill::Error>, json: bool) -> i32 {
    let done = match held {
        Ok(done) => done,
        Err(error) => return sour(&error.to_string()),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": action,
                "changed": done.kept.iter().map(|seat| serde_json::json!({
                    "agent": seat.agent,
                    "path": seat.path,
                })).collect::<Vec<_>>(),
                "unchanged": done.same.iter().map(|seat| serde_json::json!({
                    "agent": seat.agent,
                    "path": seat.path,
                })).collect::<Vec<_>>(),
                "skipped": done.left.iter().map(|skip| serde_json::json!({
                    "path": skip.path,
                    "reason": skip.note,
                })).collect::<Vec<_>>(),
            }))
            .expect("skill result should encode")
        );
    }
    for seat in &done.kept {
        if !json {
            println!("  {action} {} {}", seat.agent, seat.path.display());
        }
    }
    for seat in &done.same {
        if !json {
            println!("  unchanged {} {}", seat.agent, seat.path.display());
        }
    }
    for skip in &done.left {
        if !json {
            println!("  skipped {}: {}", skip.path.display(), skip.note);
        }
    }
    i32::from(done.kept.is_empty() && (done.same.is_empty() || !done.left.is_empty()))
}

fn report(operation: &str, held: Result<Report, plumb::skill::Error>, json: bool) -> i32 {
    let report = match held {
        Ok(report) => report,
        Err(error) => return sour(&error.to_string()),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation": operation,
                "channel": report.channel,
                "explicit": report.explicit,
                "target": report.target,
                "seats": report.seats,
            }))
            .expect("skill report should encode")
        );
    } else {
        println!("  target {} {}", report.channel, report.target.version);
        if report.seats.is_empty() {
            println!("  unmanaged");
        }
        for status in &report.seats {
            println!(
                "  {} {} -> {} {} {} {}",
                status.agent,
                status.installed,
                report.target.version,
                status.state,
                status.action,
                status.path.display()
            );
        }
    }
    i32::from(
        operation == "upgrade_dry_run"
            && (report.seats.is_empty()
                || report
                    .seats
                    .iter()
                    .any(|status| status.action == Action::Refuse)),
    )
}

fn sour(note: &str) -> i32 {
    eprintln!("plumb skill: {note}");
    1
}
