use crate::shape;

mod affirm;
mod policy;
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
        let stamped = match crate::command::changelog::identity(&held) {
            Ok(base) => crate::command::changelog::stamped(&base),
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
        let rendered = match policy::render(&self.0, &text) {
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
        let owed = match affirm::owed(&self.0) {
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
        match affirm::write(&self.0, &owed) {
            Ok(()) => {
                println!("  wrote {}", affirm::SEAT);
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
        let seat = crate::command::lane::Seat(&self.0);
        let evidence = match seat.project() {
            Ok(evidence) => evidence,
            Err(error) => {
                eprintln!("plumb lane {}: {error}", self.0.display());
                return 1;
            }
        };
        println!("plumb lane {}", self.0.display());
        println!();
        if !write {
            return Self::proposed(evidence.projected());
        }
        match seat.write(evidence.projected()) {
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

    fn proposed(lanes: &[shape::lane::Projection]) -> i32 {
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
}
