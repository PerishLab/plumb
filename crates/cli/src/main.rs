mod anchor;
mod dispatch;
mod judge;
mod rules;
mod shape;
mod skill;
mod web;

use clap::{Parser, Subcommand};
use plumb::cli::Root;
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
        #[arg(long)]
        json: bool,
    },
    Land {
        #[command(flatten)]
        target: Root,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long, default_value = "")]
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "no-watch", action = clap::ArgAction::SetFalse)]
        watch: bool,
        #[arg(long = "dry-run")]
        dry: bool,
        #[arg(long)]
        json: bool,
    },
    Precommit {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
        #[arg(long, required = true)]
        write: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    Radius {
        #[arg(long, required = true)]
        root: Vec<String>,
        #[arg(long)]
        product: String,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        json: bool,
    },
    Policy {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    Skill {
        #[command(subcommand)]
        deed: skill::Deed,
    },
    Rule {
        #[command(subcommand)]
        deed: judge::catalog::query::Deed,
    },
    Lock {
        #[command(flatten)]
        target: Root,
    },
    Changelog {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        version: Option<String>,
    },
    Release {
        #[command(subcommand)]
        deed: dispatch::release::Deed,
    },
    Stable {
        #[command(subcommand)]
        deed: dispatch::operator::Stable,
    },
    Site {
        #[command(subcommand)]
        deed: dispatch::site::Deed,
    },
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Doctor { .. } => "doctor",
            Self::Land { .. } => "land",
            Self::Precommit { .. } => "precommit",
            Self::Radius { .. } => "radius",
            Self::Policy { .. } => "policy",
            Self::Skill { .. } => "skill",
            Self::Rule { .. } => "rule",
            Self::Lock { .. } => "lock",
            Self::Changelog { .. } => "changelog",
            Self::Release { .. } => "release",
            Self::Stable { .. } => "stable",
            Self::Site { .. } => "site",
        }
    }
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

fn changelog(root: PathBuf, version: Option<String>) -> i32 {
    let held = version
        .filter(|held| !held.trim().is_empty())
        .or_else(|| shape::read(&root).version)
        .unwrap_or_default();
    println!("plumb changelog {}", root.display());
    println!();
    if held.is_empty() {
        println!("  no version to read: the repository declares none, so pass --version");
        return 1;
    }
    let seat = shape::changelog::seat(&root, &held);
    let found = shape::changelog::read(&root, &held);
    if found.is_empty() {
        println!(
            "  {} is documented in en and zh",
            shape::changelog::stamped(&held)
        );
        return 0;
    }
    println!("  {}", seat.display());
    for line in &found {
        println!("    {line}");
    }
    println!();
    println!("  a stable release is immutable; what it changed cannot be written afterwards");
    1
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

fn execute(command: Command) -> i32 {
    match command {
        Command::Doctor { target, json } => judge::doctor::run(PathBuf::from(target.root), json),
        Command::Land {
            target,
            base,
            title,
            body,
            watch,
            dry,
            json,
        } => dispatch::land::run(dispatch::land::Input {
            root: PathBuf::from(target.root),
            base,
            title,
            body,
            watch,
            dry,
            json,
        }),
        Command::Precommit {
            target,
            base,
            head,
            write,
            json,
        } => judge::precommit::run(judge::precommit::Input {
            root: PathBuf::from(target.root),
            base,
            head,
            write,
            json,
        }),
        Command::Radius {
            root,
            product,
            candidate,
            json,
        } => judge::radius::run(judge::radius::Input {
            roots: root,
            product,
            candidate,
            json,
        }),
        Command::Policy { target, write } => policy(PathBuf::from(target.root), write),
        Command::Skill { deed } => skill::run(deed),
        Command::Rule { deed } => judge::catalog::query::run(deed),
        Command::Lock { target } => locks(PathBuf::from(target.root)),
        Command::Changelog { target, version } => changelog(PathBuf::from(target.root), version),
        Command::Release { deed } => dispatch::release::run(deed),
        Command::Stable { deed } => dispatch::operator::run(deed),
        Command::Site { deed } => dispatch::site::run(deed),
    }
}

fn main() {
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command,
        Err(error) => {
            let code = error.exit_code();
            let run = dispatch::audit::Run::start("parse");
            let _ = error.print();
            if let Some(run) = run {
                run.finish(code);
            }
            std::process::exit(code);
        }
    };
    let run = dispatch::audit::Run::start(command.name());
    let code = execute(command);
    if let Some(run) = run {
        run.finish(code);
    }
    std::process::exit(code);
}
