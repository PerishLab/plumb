use crate::shape;
use std::path::PathBuf;

pub struct Seat(PathBuf);

impl Seat {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    pub fn lock(&self) -> i32 {
        let held = shape::read(&self.0);
        println!("plumb lock {}", self.0.display());
        println!();
        if held.locks.is_empty() {
            println!("  no lock is declared");
            return 0;
        }
        let seen = held.version.unwrap_or_default();
        for lock in &held.locks {
            match shape::seal(&self.0, lock) {
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

    pub fn changelog(&self, version: Option<String>) -> i32 {
        let held = version
            .filter(|held| !held.trim().is_empty())
            .or_else(|| shape::read(&self.0).version)
            .unwrap_or_default();
        println!("plumb changelog {}", self.0.display());
        println!();
        if held.is_empty() {
            println!("  no version to read: the repository declares none, so pass --version");
            return 1;
        }
        let seat = shape::changelog::seat(&self.0, &held);
        let found = shape::changelog::read(&self.0, &held);
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

    pub fn document(&self) -> i32 {
        let snapshot = plumb::snapshot::Snapshot::read(&self.0);
        let held = shape::capture(&self.0, &snapshot);
        println!("plumb document {}", self.0.display());
        println!();
        match &held.documents.config {
            shape::document::Config::Held(_) => {}
            shape::document::Config::Blind(error) | shape::document::Config::Wrong(error) => {
                println!("  {error}");
                return 1;
            }
            _ => {
                println!("  no document binding is declared");
                return 1;
            }
        }
        if let Some(error) = &held.documents.blind {
            println!("  {error}");
            return 1;
        }
        let ok = held.documents.held.iter().all(Self::proposal);
        println!();
        println!("  record these seals only after reading every named source and target");
        i32::from(!ok)
    }

    pub fn policy(&self, write: bool) -> i32 {
        let path = self.0.join("ectropy.toml");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                eprintln!("plumb policy {}: {error}", self.0.display());
                return 1;
            }
        };
        let rendered = match shape::reconcile(&self.0, &text) {
            Ok(rendered) => rendered,
            Err(error) => {
                eprintln!("plumb policy {}: {error}", self.0.display());
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
            eprintln!("plumb policy {}: {error}", self.0.display());
            return 1;
        }
        println!(
            "plumb policy {}: wrote {}",
            self.0.display(),
            path.display()
        );
        0
    }

    fn proposal(document: &shape::document::Document) -> bool {
        println!("  {} {}", document.strategy.id(), document.target);
        let mut ok = document.sources.iter().all(Self::source);
        if let Some(seal) = &document.actual {
            println!("    target-seal = \"{seal}\"");
        } else {
            ok = false;
        }
        for error in &document.errors {
            println!("    {error}");
            ok = false;
        }
        ok
    }

    fn source(source: &shape::document::Source) -> bool {
        match (&source.actual, &source.error) {
            (Some(seal), _) => {
                println!("    source {} seal = \"{seal}\"", source.path);
                true
            }
            (_, Some(error)) => {
                println!("    {error}");
                false
            }
            _ => false,
        }
    }
}
