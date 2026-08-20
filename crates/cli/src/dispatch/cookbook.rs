use std::fmt::Write;

struct Entry {
    name: &'static str,
    body: &'static str,
}

const ENTRIES: &[Entry] = &[
    Entry {
        name: "affirmed",
        body: plumb::seat::resource!("cookbook/affirmed.txt"),
    },
    Entry {
        name: "seat",
        body: plumb::seat::resource!("cookbook/seat.txt"),
    },
    Entry {
        name: "wayfinder",
        body: plumb::seat::resource!("cookbook/wayfinder.txt"),
    },
];

pub fn run(name: Option<String>) -> i32 {
    match render(name.as_deref()) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("plumb cookbook: {error}");
            1
        }
    }
}

fn render(name: Option<&str>) -> Result<String, String> {
    let Some(name) = name else {
        return Ok(ledger());
    };
    ENTRIES
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.body.to_string())
        .ok_or_else(|| format!("unknown cookbook entry `{name}`; available: {}", names()))
}

fn ledger() -> String {
    let mut held = String::new();
    for entry in ENTRIES {
        let _ = writeln!(held, "{}", entry.name);
        for line in entry.body.lines() {
            if let Some(exit) = line.strip_prefix("Remove this entry ") {
                let _ = writeln!(held, "  EXIT: Remove this entry {exit}");
            }
        }
    }
    held
}

fn names() -> String {
    ENTRIES
        .iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>()
        .join(", ")
}
