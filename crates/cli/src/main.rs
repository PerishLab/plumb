mod anchor;
mod dispatch;
mod judge;
mod pack;
mod rules;
mod shape;
mod web;

use clap::{Parser, Subcommand};
use judge::{judge, show};
use plumb::cli::Root;
use plumb::rig::Rig;
use plumb::skill::{Ask, Done, Kit};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "plumb", version = plumb::version!("PLUMB"))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Doctor {
        #[command(flatten)]
        target: Root,
    },
    Policy {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    Skill {
        #[command(subcommand)]
        deed: Deed,
    },
    Lock {
        #[command(flatten)]
        target: Root,
    },
}

fn doctor(root: PathBuf) -> i32 {
    let held = shape::read(&root);
    println!("plumb doctor {}", root.display());
    println!();
    println!("  wrappers  {}", show(&held.wrappers));
    println!("  layout    {}", show(&held.dirs));
    println!("  lanes     {}", show(&held.lanes));
    println!("  publishes {}", show(&held.ships));
    println!(
        "  law       block={} path={} grants={}",
        held.block.unwrap_or(0),
        held.path.unwrap_or(0),
        show(&held.grants)
    );
    println!();
    let notes = judge(&held);
    if notes.is_empty() {
        println!("  true to the skeleton");
        return 0;
    }
    let mut wrong = 0;
    let mut blind = 0;
    for note in &notes {
        println!("  {}: {} [{}]", note.grade, note.line, note.dim);
        if note.grade == "out of true" {
            wrong += 1;
        }
        if note.grade == "blind" {
            blind += 1;
        }
    }
    println!();
    println!(
        "  {wrong} out of true, {} unknown to the skeleton, {blind} blind",
        notes.len() - wrong - blind
    );
    i32::from(wrong > 0)
}

fn locks(root: PathBuf) -> i32 {
    let held = shape::read(&root);
    println!("plumb lock {}", root.display());
    println!();
    if held.locks.is_empty() {
        println!("  no lock is declared");
        return 0;
    }
    let seen = held.version.unwrap_or_default();
    for lock in &held.locks {
        match shape::seal(&root, lock) {
            Ok(hash) => println!(
                "  {} version = \"{seen}\"\n  {} hash = \"{hash}\"",
                lock.name, lock.name
            ),
            Err(why) => println!("  {} {why}", lock.name),
        }
    }
    println!();
    println!("  record these in plumb.toml only after reading what they cover");
    0
}

fn policy(root: PathBuf, write: bool) -> i32 {
    let path = root.join("ectropy.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            eprintln!("plumb policy {}: {error}", root.display());
            return 1;
        }
    };
    let rendered = match shape::reconcile(&root, &text) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("plumb policy {}: {error}", root.display());
            return 1;
        }
    };
    if !write {
        print!("{rendered}");
        return 0;
    }
    let draft = path.with_extension(format!("toml.tmp-{}", std::process::id()));
    if let Err(error) =
        std::fs::write(&draft, rendered).and_then(|()| std::fs::rename(&draft, &path))
    {
        let _ = std::fs::remove_file(&draft);
        eprintln!("plumb policy {}: {error}", root.display());
        return 1;
    }
    println!("plumb policy {}: wrote {}", root.display(), path.display());
    0
}

fn main() {
    let code = match Cli::parse().command {
        Command::Doctor { target } => doctor(PathBuf::from(target.root)),
        Command::Policy { target, write } => policy(PathBuf::from(target.root), write),
        Command::Skill { deed } => skill(deed),
        Command::Lock { target } => locks(PathBuf::from(target.root)),
    };
    std::process::exit(code);
}

#[derive(Subcommand)]
enum Deed {
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
    Upgrade {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
    },
    List,
    Uninstall,
}

fn skill(deed: Deed) -> i32 {
    let rig = match Rig::resolve(None) {
        Ok(rig) => rig,
        Err(error) => return sour(&error.to_string()),
    };
    if rig.home.is_empty() {
        return sour("no data home; set PLUMB_HOME");
    }
    let kit = Kit {
        name: "plumb".to_string(),
        home: seat(),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases.clone(),
    };
    run(&kit, deed)
}

fn run(kit: &Kit, deed: Deed) -> i32 {
    match deed {
        Deed::Install {
            channel,
            version,
            path,
            force,
        } => told(kit.install(&Ask {
            channel,
            version,
            path,
            force,
        })),
        Deed::Upgrade { channel, version } => told(kit.upgrade(&Ask {
            channel,
            version,
            ..Ask::default()
        })),
        Deed::List => tell(kit),
        Deed::Uninstall => told(kit.uninstall()),
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

fn told(held: Result<Done, plumb::skill::Error>) -> i32 {
    let done = match held {
        Ok(done) => done,
        Err(error) => return sour(&error.to_string()),
    };
    for seat in &done.kept {
        println!("  {} {}", seat.agent, seat.path.display());
    }
    for skip in &done.left {
        println!("  skipped {}: {}", skip.path.display(), skip.note);
    }
    i32::from(done.kept.is_empty())
}

fn seat() -> PathBuf {
    plumb::config::home().unwrap_or_else(|| PathBuf::from("."))
}

fn sour(note: &str) -> i32 {
    eprintln!("plumb skill: {note}");
    1
}
