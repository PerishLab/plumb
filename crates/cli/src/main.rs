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

mod audit {
    use locus::collector;
    use locus::generator;
    use locus::reporter;
    use locus::{Candidate, Config, Context, Engine, Key, Policy, Role};
    use serde_json::json;

    const TARGET: &str = "target";
    const LIMIT: usize = 512;

    pub(crate) struct Run {
        engine: Engine,
        context: Context,
        command: &'static str,
    }

    impl Run {
        pub(crate) fn start(command: &'static str) -> Option<Self> {
            let (engine, explicit, target) = bootstrap()?;
            let mut candidate = Candidate::event(json!({
                "event": "cli.start",
                "command": command,
            }))
            .ensure(Role::trace())
            .ensure(Role::span());
            if let Some(role) = target {
                candidate = candidate.collect(role, TARGET);
            }
            if let Some(key) = explicit {
                candidate = candidate.explicit(Role::trace(), key);
            }
            let accepted = locus::record!(&engine, &Context::empty(), candidate).ok()?;
            Some(Self {
                engine,
                context: accepted.context(),
                command,
            })
        }

        pub(crate) fn finish(self, code: i32) {
            let candidate = Candidate::event(json!({
                "event": "cli.finish",
                "command": self.command,
                "code": code,
            }))
            .ensure(Role::trace())
            .ensure(Role::span());
            let _ = locus::record!(&self.engine, &self.context, candidate);
        }
    }

    fn bootstrap() -> Option<(Engine, Option<Key>, Option<Role>)> {
        let mut policy = Policy::default();
        if let Some(path) = plumb::config::value("PLUMB_LOCUS_TRACE_FILE") {
            policy = policy.generator(Role::trace(), generator::Spec::shared(path));
        }
        if let Some(path) = plumb::config::value("PLUMB_LOCUS_REPORT_FILE") {
            policy = policy.reporter(reporter::Spec::file(path));
        }
        let explicit = plumb::config::value("PLUMB_LOCUS_TRACE_ID")
            .map(Key::new)
            .transpose()
            .ok()?;
        let target = plumb::config::value("PLUMB_LOCUS_TARGET_COLLECTORS");
        let role = target
            .as_ref()
            .map(|_| Role::new("plumb.target"))
            .transpose()
            .ok()?;
        for held in target.iter().flat_map(|value| value.split(',')) {
            policy = policy.collector(TARGET, spec(held.trim()));
        }
        Engine::bootstrap(Config::new(policy))
            .ok()
            .map(|engine| (engine, explicit, role))
    }

    fn spec(value: &str) -> collector::Spec {
        let Some((name, selector)) = value.split_once(':') else {
            return collector::Spec::new(value, json!({}));
        };
        match name {
            "environment" => collector::Spec::environment(selector, LIMIT),
            "argv" => match selector.parse() {
                Ok(index) => collector::Spec::argument(index, LIMIT),
                Err(_) => collector::Spec::new("argv", json!({"index": selector, "limit": LIMIT})),
            },
            "process" => collector::Spec::process(selector, LIMIT),
            _ => collector::Spec::new(name, json!({})),
        }
    }
}

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
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Doctor { .. } => "doctor",
            Self::Policy { .. } => "policy",
            Self::Skill { .. } => "skill",
            Self::Rule { .. } => "rule",
            Self::Lock { .. } => "lock",
            Self::Changelog { .. } => "changelog",
            Self::Release { .. } => "release",
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
        Command::Policy { target, write } => policy(PathBuf::from(target.root), write),
        Command::Skill { deed } => skill::run(deed),
        Command::Rule { deed } => judge::catalog::query::run(deed),
        Command::Lock { target } => locks(PathBuf::from(target.root)),
        Command::Changelog { target, version } => changelog(PathBuf::from(target.root), version),
        Command::Release { deed } => dispatch::release::run(deed),
    }
}

fn main() {
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command,
        Err(error) => {
            let code = error.exit_code();
            let run = audit::Run::start("parse");
            let _ = error.print();
            if let Some(run) = run {
                run.finish(code);
            }
            std::process::exit(code);
        }
    };
    let run = audit::Run::start(command.name());
    let code = execute(command);
    if let Some(run) = run {
        run.finish(code);
    }
    std::process::exit(code);
}
