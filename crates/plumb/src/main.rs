mod shape;

use clap::{Parser, Subcommand};
use std::collections::BTreeSet;
use std::path::PathBuf;

struct Note {
    grade: &'static str,
    dim: &'static str,
    line: String,
}

type Found = Vec<(&'static str, String)>;

#[derive(Parser)]
#[command(name = "plumb", version = concat!("v", env!("CARGO_PKG_VERSION")))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Doctor {
        #[arg(default_value = ".")]
        path: String,
    },
}

const DIRS: [&str; 7] = [
    "apps", "charts", "crates", "docs", "lib", "packages", "tests",
];

const WRAPPERS: [&str; 6] = ["guard", "init", "land", "release", "ship", "bake"];

const LANES: [&str; 4] = ["guard", "release-beta", "release-stable", "probe"];

const GUARD_CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";

const CI_CONTAINER: &str = "mirror.perish.lan/ci/deno";

fn judge(held: &shape::Shape) -> Vec<Note> {
    let mut notes = Vec::new();
    for (dim, found) in [
        ("env", env(held)),
        ("structure", structure(held)),
        ("deps", deps(held)),
    ] {
        for (grade, line) in found {
            notes.push(Note { grade, dim, line });
        }
    }
    notes
}

fn env(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held
        .edition
        .as_ref()
        .is_some_and(|edition| edition != "2024")
    {
        found.push((
            "out of true",
            format!(
                "edition is {}, the skeleton holds 2024",
                held.edition.as_deref().unwrap_or("")
            ),
        ));
    }
    let pinned = format!("{CI_CONTAINER}:");
    if held
        .guard_lane
        .as_ref()
        .is_some_and(|yml| yml.contains(&pinned))
    {
        found.push((
            "out of true",
            "CI container pinned to a tag, the skeleton tracks latest".to_string(),
        ));
    }
    found
}

fn structure(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.runseal {
        for name in ["guard", "init", "land"] {
            if !held.wrappers.contains(name) {
                found.push(("out of true", format!("no {name} wrapper")));
            }
        }
        if !held.laws {
            found.push(("out of true", "no negentropy.toml".to_string()));
        }
        if held
            .guard_lane
            .as_ref()
            .is_some_and(|yml| !yml.contains(GUARD_CONCURRENCY))
        {
            found.push((
                "out of true",
                "guard lane without the concurrency block".to_string(),
            ));
        }
    }
    if let Some(why) = &held.unread {
        found.push((
            "blind",
            format!(
                "cannot read negentropy.toml: {}",
                why.lines().next().unwrap_or("")
            ),
        ));
    } else {
        for (key, seen, want) in [("block", held.block, 4), ("path", held.path, 4)] {
            if seen.is_some_and(|value| value != want) {
                found.push((
                    "out of true",
                    format!(
                        "limit {key} is {}, the skeleton holds {want}",
                        seen.unwrap_or(0)
                    ),
                ));
            }
        }
        if held.laws && !held.grants.contains("test") {
            found.push(("out of true", "no test grant".to_string()));
        }
    }
    for name in &held.dirs {
        if !DIRS.contains(&name.as_str()) {
            found.push((
                "unknown shape",
                format!("directory {name} has no shadow in the skeleton"),
            ));
        }
    }
    for name in &held.wrappers {
        if !WRAPPERS.contains(&name.as_str()) {
            found.push((
                "unknown shape",
                format!("wrapper {name} has no shadow in the skeleton"),
            ));
        }
    }
    paired(held, &mut found);
    matched(held, &mut found);
    for name in &held.lanes {
        if !LANES.contains(&name.as_str()) {
            found.push((
                "unknown shape",
                format!("workflow {name} has no shadow in the skeleton"),
            ));
        }
    }
    found
}

fn deps(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.binary && !held.clap {
        found.push((
            "out of true",
            "ships a rust binary without clap".to_string(),
        ));
    }
    found
}

fn paired(held: &shape::Shape, found: &mut Found) {
    if held.rust && !held.ignore.lines().any(|line| line.trim() == "target/") {
        found.push((
            "out of true",
            "Cargo.toml without target/ in .gitignore".to_string(),
        ));
    }
    if held.wrappers.contains("release") {
        for lane in ["release-beta", "release-stable"] {
            if !held.lanes.contains(lane) {
                found.push((
                    "out of true",
                    format!("release wrapper without a {lane} lane"),
                ));
            }
        }
    }
    if !held.ships.is_empty() && !held.wrappers.contains("release") {
        found.push((
            "out of true",
            format!("declares {} without a release wrapper", show(&held.ships)),
        ));
    }
}

fn matched(held: &shape::Shape, found: &mut Found) {
    if held.inits && held.listed.is_empty() {
        found.push((
            "blind",
            "an init wrapper is present but its required paths could not be read".to_string(),
        ));
    }
    for name in held.listed.difference(&held.wrappers) {
        found.push((
            "out of true",
            format!("init requires wrapper {name} which does not exist"),
        ));
    }
    for path in &held.bounds {
        if !held.root.join(path).exists() {
            found.push((
                "out of true",
                format!("boundary names {path} which does not exist"),
            ));
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

fn main() {
    let code = match Cli::parse().command {
        Command::Doctor { path } => doctor(PathBuf::from(path)),
    };
    std::process::exit(code);
}
