use crate::shape;
use std::path::PathBuf;

pub struct Seat(PathBuf);

impl Seat {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    pub fn changelog(&self, version: Option<String>) -> i32 {
        let held = version
            .filter(|held| !held.trim().is_empty())
            .or_else(|| shape::version(&self.0))
            .unwrap_or_default();
        println!("plumb changelog {}", self.0.display());
        println!();
        if held.is_empty() {
            println!("  no version to read: the repository declares none, so pass --version");
            return 1;
        }
        let stamped = match shape::changelog::identity(&held) {
            Ok(base) => shape::changelog::stamped(&base),
            Err(error) => {
                println!("  {error}");
                return 1;
            }
        };
        match super::depot::occupied(&stamped) {
            Err(error) => {
                println!("  {error}");
                1
            }
            Ok(None) => {
                println!("  the depot carries no release note for {stamped}");
                println!();
                println!(
                    "  write one under .tmp/plumb/changelog/{stamped} and run plumb depot changelog"
                );
                1
            }
            Ok(Some(notes)) => {
                println!(
                    "  {stamped} is documented in {} objects",
                    notes.objects.len()
                );
                for object in &notes.objects {
                    println!("    {}", object.path);
                }
                0
            }
        }
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

    pub fn affirm(&self, write: bool) -> i32 {
        let owed = match crate::shape::layout::affirm::owed(&self.0) {
            Ok(owed) => owed,
            Err(error) => {
                eprintln!("plumb affirm {}: {error}", self.0.display());
                return 1;
            }
        };
        println!("plumb affirm {}", self.0.display());
        println!();
        if owed.is_empty() {
            println!("  no declared rule asks for an affirmation");
            return 0;
        }
        for held in &owed {
            println!("  {} {}", held.rule, held.target);
            println!("    authority = \"{}\"", held.authority);
        }
        println!();
        if !write {
            println!("  record these only after reading every target against what moved");
            return 0;
        }
        match crate::shape::layout::affirm::write(&self.0, &owed) {
            Ok(()) => {
                println!("  wrote {}", crate::shape::layout::affirm::SEAT);
                0
            }
            Err(error) => {
                eprintln!("plumb affirm {}: {error}", self.0.display());
                1
            }
        }
    }

    pub fn layout(&self) -> i32 {
        let held = crate::shape::layout::stated(&self.0);
        println!("plumb layout {}", self.0.display());
        println!();
        match &held {
            crate::shape::layout::Held::Outside => {
                println!("  no plumb.toml selects this root");
                1
            }
            crate::shape::layout::Held::Wrong(error) => {
                println!("  {error}");
                1
            }
            crate::shape::layout::Held::Absent => {
                println!("  no layout is declared; the released name sets still judge this root");
                0
            }
            crate::shape::layout::Held::Stated(declared) => {
                for seat in &declared.seats {
                    println!("  {}", seat.shown());
                }
                for group in &declared.groups {
                    println!("  {}", group.shown());
                }
                0
            }
        }
    }

    pub fn lane(&self, write: bool) -> i32 {
        let seat = shape::lane::Seat(&self.0);
        let lanes = match seat.render() {
            Ok(lanes) => lanes,
            Err(error) => {
                eprintln!("plumb lane {}: {error}", self.0.display());
                return 1;
            }
        };
        println!("plumb lane {}", self.0.display());
        println!();
        if !write {
            return Self::proposed(&lanes);
        }
        match seat.write(&lanes) {
            Ok(written) if written.is_empty() => {
                println!("  every rendered lane already stands as written");
                0
            }
            Ok(written) => {
                for path in written {
                    println!("  wrote {path}");
                }
                0
            }
            Err(error) => {
                eprintln!("plumb lane {}: {error}", self.0.display());
                1
            }
        }
    }

    fn proposed(lanes: &[shape::lane::Lane]) -> i32 {
        let mut drifted = 0;
        for lane in lanes {
            if !lane.drifted() {
                println!("  true {}", lane.path);
                continue;
            }
            drifted += 1;
            let state = if lane.absent() { "absent" } else { "drifted" };
            println!("  {state} {}", lane.path);
        }
        if drifted == 0 {
            return 0;
        }
        println!();
        println!("  run plumb lane --write to render what this repository declares");
        1
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
        if !source.loose.is_empty() {
            println!(
                "    source {} holds untracked leaves; git add or ignore them first: {}",
                source.path,
                shape::document::named(&source.loose)
            );
            return false;
        }
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
