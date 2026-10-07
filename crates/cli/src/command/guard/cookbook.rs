use plumb::cookbook::{Cookbook, Entry};
use std::fmt::Write;

const SOURCES: &[&str] = &[
    plumb::seat::resource!("cookbook/env.toolchain-domain.txt"),
    plumb::seat::resource!("cookbook/guard.integration-branch.txt"),
    plumb::seat::resource!("cookbook/structure.known-directory.txt"),
    plumb::seat::resource!("cookbook/structure.known-file.txt"),
    plumb::seat::resource!("cookbook/structure.seat-affirmed.txt"),
    plumb::seat::resource!("cookbook/structure.seat-anchored.txt"),
    plumb::seat::resource!("cookbook/structure.seat-member.txt"),
];

pub fn run(code: Option<String>, json: bool) -> i32 {
    match render(code.as_deref(), json) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            eprintln!("plumb cookbook: {error}");
            1
        }
    }
}

fn render(code: Option<&str>, json: bool) -> Result<String, String> {
    let book = book()?;
    let Some(code) = code else {
        return if json {
            serde_json::to_string_pretty(&book).map_err(|error| error.to_string())
        } else {
            Ok(ledger(&book))
        };
    };
    let entry = book.get(code).ok_or_else(|| {
        format!(
            "unknown Cookbook code `{code}`; available: {}",
            names(&book)
        )
    })?;
    if json {
        serde_json::to_string_pretty(entry).map_err(|error| error.to_string())
    } else {
        Ok(page(entry))
    }
}

fn book() -> Result<Cookbook, String> {
    let entries = SOURCES
        .iter()
        .map(|source| Entry::parse(source))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Cookbook::new(entries).map_err(|error| error.to_string())
}

fn ledger(book: &Cookbook) -> String {
    let mut held = String::new();
    for entry in book.entries() {
        let _ = writeln!(held, "{}", entry.code());
        if let Some(exit) = entry.exit() {
            let _ = writeln!(held, "  EXIT: {exit}");
        }
    }
    held.trim_end().to_string()
}

fn page(entry: &Entry) -> String {
    let mut held = format!("# {}", entry.code());
    section(&mut held, "Trigger", entry.trigger());
    section(&mut held, "Solution", entry.solution());
    if let Some(evidence) = entry.evidence() {
        section(&mut held, "Evidence", evidence);
    }
    if let Some(exit) = entry.exit() {
        section(&mut held, "EXIT", exit);
    }
    held
}

fn section(page: &mut String, name: &str, body: &str) {
    let _ = write!(page, "\n\n## {name}\n\n{body}");
}

fn names(book: &Cookbook) -> String {
    book.entries()
        .iter()
        .map(|entry| entry.code().text())
        .collect::<Vec<_>>()
        .join(", ")
}
