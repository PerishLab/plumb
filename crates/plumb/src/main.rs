mod shape;

use std::collections::BTreeSet;
use std::path::PathBuf;

struct Note {
    grade: &'static str,
    line: String,
}

fn usage() {
    println!("Usage: plumb doctor [path]");
    println!();
    println!("Hold a repository against the skeleton and report where it hangs");
    println!("out of true. plumb reports; it never edits. A finding may be the");
    println!("repository's debt or the skeleton's own.");
}

const DIRS: [&str; 7] = [
    "apps", "charts", "crates", "docs", "lib", "packages", "tests",
];

const WRAPPERS: [&str; 6] = ["guard", "init", "land", "release", "ship", "bake"];

const LANES: [&str; 4] = ["guard", "release-beta", "release-stable", "probe"];

const GUARD_CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";

type Layer = fn(&shape::Shape, &mut Vec<Note>);

const LAYERS: [Layer; 3] = [env, structure, deps];

fn judge(held: &shape::Shape) -> Vec<Note> {
    let mut notes = Vec::new();
    if let Some(why) = &held.unread {
        notes.push(Note {
            grade: "blind",
            line: format!(
                "cannot read negentropy.toml: {}",
                why.lines().next().unwrap_or("")
            ),
        });
        return notes;
    }
    for layer in LAYERS {
        layer(held, &mut notes);
        if notes.iter().any(|note| note.grade == "out of true") {
            break;
        }
    }
    notes
}

fn env(_held: &shape::Shape, _notes: &mut Vec<Note>) {}

fn structure(held: &shape::Shape, notes: &mut Vec<Note>) {
    if held.runseal {
        for name in ["guard", "init", "land"] {
            if !held.wrappers.contains(name) {
                notes.push(Note {
                    grade: "out of true",
                    line: format!("no {name} wrapper"),
                });
            }
        }
        if !held.laws {
            notes.push(Note {
                grade: "out of true",
                line: "no negentropy.toml".to_string(),
            });
        }
        if held
            .guard_lane
            .as_ref()
            .is_some_and(|yml| !yml.contains(GUARD_CONCURRENCY))
        {
            notes.push(Note {
                grade: "out of true",
                line: "guard lane without the concurrency block".to_string(),
            });
        }
    }
    for (key, seen, want) in [("block", held.block, 4), ("path", held.path, 4)] {
        if seen.is_some_and(|value| value != want) {
            notes.push(Note {
                grade: "out of true",
                line: format!(
                    "limit {key} is {}, the skeleton holds {want}",
                    seen.unwrap_or(0)
                ),
            });
        }
    }
    if held.laws && !held.grants.contains("test") {
        notes.push(Note {
            grade: "out of true",
            line: "no test grant".to_string(),
        });
    }
    for name in &held.dirs {
        if !DIRS.contains(&name.as_str()) {
            notes.push(Note {
                grade: "unknown shape",
                line: format!("directory {name} has no shadow in the skeleton"),
            });
        }
    }
    for name in &held.wrappers {
        if !WRAPPERS.contains(&name.as_str()) {
            notes.push(Note {
                grade: "unknown shape",
                line: format!("wrapper {name} has no shadow in the skeleton"),
            });
        }
    }
    paired(held, notes);
    matched(held, notes);
    for name in &held.lanes {
        if !LANES.contains(&name.as_str()) {
            notes.push(Note {
                grade: "unknown shape",
                line: format!("workflow {name} has no shadow in the skeleton"),
            });
        }
    }
}

fn deps(_held: &shape::Shape, _notes: &mut Vec<Note>) {}

fn paired(held: &shape::Shape, notes: &mut Vec<Note>) {
    if held.rust && !held.ignore.lines().any(|line| line.trim() == "target/") {
        notes.push(Note {
            grade: "out of true",
            line: "Cargo.toml without target/ in .gitignore".to_string(),
        });
    }
    if held.wrappers.contains("release") {
        for lane in ["release-beta", "release-stable"] {
            if !held.lanes.contains(lane) {
                notes.push(Note {
                    grade: "out of true",
                    line: format!("release wrapper without a {lane} lane"),
                });
            }
        }
    }
    if !held.ships.is_empty() && !held.wrappers.contains("release") {
        notes.push(Note {
            grade: "out of true",
            line: format!("declares {} without a release wrapper", show(&held.ships)),
        });
    }
}

fn matched(held: &shape::Shape, notes: &mut Vec<Note>) {
    if held.inits && held.listed.is_empty() {
        notes.push(Note {
            grade: "blind",
            line: "an init wrapper is present but its required paths could not be read".to_string(),
        });
    }
    for name in held.listed.difference(&held.wrappers) {
        notes.push(Note {
            grade: "out of true",
            line: format!("init requires wrapper {name} which does not exist"),
        });
    }
    for path in &held.bounds {
        if !held.root.join(path).exists() {
            notes.push(Note {
                grade: "out of true",
                line: format!("boundary names {path} which does not exist"),
            });
        }
    }
}

fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
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
        println!("  {}: {}", note.grade, note.line);
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

fn main() {
    let mut args = std::env::args().skip(1);
    let code = match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("plumb v{}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("doctor") => {
            let seat = args.next().unwrap_or_else(|| ".".to_string());
            doctor(PathBuf::from(seat))
        }
        _ => {
            usage();
            0
        }
    };
    std::process::exit(code);
}
