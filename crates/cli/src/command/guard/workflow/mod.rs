mod plan;
mod remote;
mod reuse;
mod spread;
mod tree;

use crate::shape;
use clap::Subcommand;
use plumb::cli::Root;
use plumb::rig::Rig;
use remote::{Remote, SEAT};
use spread::spread;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Report what the recorded runs of this repository's lanes did")]
    Status {
        #[command(flatten)]
        target: Root,
        #[arg(long, default_value = "")]
        since: String,
    },
    #[command(about = "Ask, per key, whether a lane may skip the step that key owns")]
    Ask {
        lane: String,
        #[command(flatten)]
        target: Root,
    },
    #[command(about = "Plan which declared actions moved between one base and the current tree")]
    Plan(plan::Input),
    #[command(about = "Print the input hash one declared key resolves to")]
    Hash {
        key: String,
        #[command(flatten)]
        target: Root,
    },
    #[command(about = "Record that one key already ran, beside the release objects")]
    Lock {
        key: String,
        #[command(flatten)]
        target: Root,
    },
}

pub fn run(deed: Deed) -> i32 {
    match deed {
        Deed::Status { target, since } => Seat::new(target).status(since.trim()),
        Deed::Ask { lane, target } => Seat::new(target).ask(&lane),
        Deed::Plan(input) => plan::run(input),
        Deed::Hash { key, target } => Seat::new(target).compare(&key),
        Deed::Lock { key, target } => Seat::new(target).record(&key),
    }
}

struct Seat(PathBuf);

impl Seat {
    fn new(target: Root) -> Self {
        Self(PathBuf::from(target.root))
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn status(&self, since: &str) -> i32 {
        let held = shape::workflow::read(self.root());
        println!("plumb workflow {}", self.0.display());
        println!();
        if let Some(error) = &held.refusal {
            println!("  {error}");
            return 1;
        }
        if held.keys.is_empty() {
            println!("  no workflow hash is declared");
            return 1;
        }
        if since.is_empty() {
            return self.current(&held);
        }
        self.replay(&held, since)
    }

    fn current(&self, held: &shape::workflow::Held) -> i32 {
        let tree = match tree::Tree::read(self.root(), None) {
            Ok(tree) => tree,
            Err(error) => {
                println!("  {error}");
                return 1;
            }
        };
        for key in &held.keys {
            println!(
                "  {:<16} {}  {:>2} paths  {:>5} leaves",
                key.name(),
                &tree.digest(key)[..12],
                key.paths.len(),
                tree.covered(key)
            );
            let mut plain = Vec::new();
            for root in &key.roots {
                match spread(held, root) {
                    Some(shown) => println!("    {root:<20} {shown}"),
                    None => plain.push(root.as_str()),
                }
            }
            if !plain.is_empty() {
                println!("    {}", plain.join(" "));
            }
        }
        0
    }

    fn replay(&self, held: &shape::workflow::Held, since: &str) -> i32 {
        let commits = match tree::Git::new(self.root()).history(since) {
            Ok(commits) => commits,
            Err(error) => {
                println!("  {error}");
                return 1;
            }
        };
        if commits.len() < 2 {
            println!("  {since}..HEAD holds too few commits to replay");
            return 1;
        }
        let mut prior: Vec<String> = Vec::new();
        let mut kept = vec![0usize; held.keys.len()];
        for commit in &commits {
            let tree = match tree::Tree::read(self.root(), Some(commit)) {
                Ok(tree) => tree,
                Err(error) => {
                    println!("  {error}");
                    return 1;
                }
            };
            let found: Vec<String> = held.keys.iter().map(|key| tree.digest(key)).collect();
            for (at, digest) in found.iter().enumerate() {
                if prior.get(at).is_some_and(|held| held == digest) {
                    kept[at] += 1;
                }
            }
            prior = found;
        }
        let total = commits.len() - 1;
        println!("  {} commits replayed from {since}", commits.len());
        println!();
        for (at, key) in held.keys.iter().enumerate() {
            println!(
                "  {:<16} {}/{total} unchanged ({}%)",
                key.name(),
                kept[at],
                kept[at] * 100 / total
            );
        }
        0
    }

    fn digest(&self, key: &str) -> Result<String, String> {
        let held = shape::workflow::read(self.root());
        if let Some(error) = held.refusal {
            return Err(error);
        }
        let found = held
            .keys
            .iter()
            .find(|held| held.name() == key)
            .ok_or_else(|| format!("no key called {key} is declared"))?;
        let tree = tree::Tree::read(self.root(), None)?;
        Ok(tree.digest(found))
    }

    fn seat(rig: &Rig, key: &str) -> Result<String, String> {
        let held = rig.workflow.seat.trim();
        if held.is_empty() {
            return Err(format!(
                "{key} names no workflow seat; only a run may read or record a lock"
            ));
        }
        Ok(format!("{SEAT}/{held}/{key}"))
    }

    fn ask(&self, lane: &str) -> i32 {
        let held = shape::workflow::read(self.root());
        let listed: Vec<&shape::workflow::Key> =
            held.keys.iter().filter(|key| key.lane() == lane).collect();
        if listed.is_empty() {
            return 0;
        }
        let rig = match Rig::resolve(None) {
            Ok(rig) => rig,
            Err(error) => {
                eprintln!("plumb workflow ask {lane}: {error}");
                Rig::default()
            }
        };
        let tree = tree::Tree::read(self.root(), None).ok();
        for key in listed {
            println!("{}={}", key.output(), self.runs(&rig, key, tree.as_ref()));
        }
        0
    }

    fn runs(&self, rig: &Rig, key: &shape::workflow::Key, tree: Option<&tree::Tree>) -> bool {
        if rig.workflow.force {
            return true;
        }
        let Some(tree) = tree else {
            return true;
        };
        let digest = tree.digest(key);
        let Ok(seat) = Self::seat(rig, &key.name()) else {
            return true;
        };
        !matches!(
            Remote::new(&rig.lock).read(&seat),
            Ok(Some(seen)) if seen == digest
        )
    }

    fn compare(&self, key: &str) -> i32 {
        let rig = match Rig::resolve(None) {
            Ok(rig) => rig,
            Err(error) => {
                eprintln!("plumb workflow hash {key}: {error}");
                println!("{key} runs: the rig is unreadable");
                return 1;
            }
        };
        let digest = match self.digest(key) {
            Ok(digest) => digest,
            Err(error) => {
                eprintln!("plumb workflow hash {key}: {error}");
                return 1;
            }
        };
        if rig.workflow.force {
            println!("{key} runs: the run forces every step");
            return 1;
        }
        let seat = match Self::seat(&rig, key) {
            Ok(seat) => seat,
            Err(error) => {
                eprintln!("plumb workflow hash {key}: {error}");
                println!("{key} runs: this run names no lock seat");
                return 1;
            }
        };
        match Remote::new(&rig.lock).read(&seat) {
            Ok(Some(seen)) if seen == digest => {
                println!("{key} holds at {}", &digest[..12]);
                0
            }
            Ok(_) => {
                println!("{key} runs: input moved to {}", &digest[..12]);
                1
            }
            Err(error) => {
                eprintln!("plumb workflow hash {key}: {error}");
                println!("{key} runs: the lock is unreadable");
                1
            }
        }
    }

    fn record(&self, key: &str) -> i32 {
        let rig = match Rig::resolve(None) {
            Ok(rig) => rig,
            Err(error) => {
                eprintln!("plumb workflow lock {key}: {error}");
                return 0;
            }
        };
        let digest = match self.digest(key) {
            Ok(digest) => digest,
            Err(error) => {
                eprintln!("plumb workflow lock {key}: {error}");
                return 0;
            }
        };
        let seat = match Self::seat(&rig, key) {
            Ok(seat) => seat,
            Err(error) => {
                eprintln!("plumb workflow lock {key}: {error}");
                return 0;
            }
        };
        match Remote::new(&rig.lock).write(&seat, &digest) {
            Ok(()) => println!("{key} locked at {}", &digest[..12]),
            Err(error) => eprintln!("plumb workflow lock {key}: {error}"),
        }
        0
    }
}
