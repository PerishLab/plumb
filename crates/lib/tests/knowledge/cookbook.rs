use plumb::cookbook::{Cookbook, Entry, Error};

fn entry(code: &str) -> Entry {
    Entry::new(code, "The operation refused.", "Read the named authority.").expect("valid entry")
}

#[test]
fn codes() {
    for invalid in ["", "seat", "Seat.missing", "seat..missing", "seat.missing!"] {
        assert!(matches!(
            Entry::new(invalid, "trigger", "solution"),
            Err(Error::Code(_))
        ));
    }
    assert!(matches!(
        Entry::new("plumb.seat-missing", " ", "solution"),
        Err(Error::Blank {
            field: "trigger",
            ..
        })
    ));
    assert!(matches!(
        Entry::new("plumb.seat-missing", "trigger", "\n"),
        Err(Error::Blank {
            field: "solution",
            ..
        })
    ));
}

#[test]
fn optional() {
    let held = entry("plumb.seat-missing")
        .observe("Measured in the repository.")
        .expect("evidence")
        .retire("Remove when the finding states the move.")
        .expect("exit");
    assert_eq!(held.evidence(), Some("Measured in the repository."));
    assert_eq!(
        held.exit(),
        Some("Remove when the finding states the move.")
    );
    assert!(matches!(held.observe(" "), Err(Error::Blank { .. })));
}

#[test]
fn ordered() {
    let book = Cookbook::new([
        entry("plumb.wayfinder-over-cap"),
        entry("plumb.seat-missing"),
    ])
    .expect("cookbook");
    let codes = book
        .entries()
        .iter()
        .map(|held| held.code().text())
        .collect::<Vec<_>>();
    assert_eq!(codes, ["plumb.seat-missing", "plumb.wayfinder-over-cap"]);
    assert_eq!(
        book.get("plumb.seat-missing")
            .expect("exact entry")
            .solution(),
        "Read the named authority."
    );
    assert!(book.get("seat-missing").is_none());
}

#[test]
fn duplicates() {
    let duplicate = Cookbook::new([entry("plumb.seat-missing"), entry("plumb.seat-missing")]);
    assert_eq!(
        duplicate,
        Err(Error::Duplicate("plumb.seat-missing".to_string()))
    );
}

#[test]
fn serializes() {
    let book = Cookbook::new([entry("plumb.seat-missing")
        .observe("Observed evidence.")
        .expect("evidence")
        .retire("Remove with the law.")
        .expect("exit")])
    .expect("cookbook");
    let value = serde_json::to_value(book).expect("json");
    assert_eq!(value["entries"][0]["code"], "plumb.seat-missing");
    assert_eq!(value["entries"][0]["trigger"], "The operation refused.");
    assert_eq!(value["entries"][0]["solution"], "Read the named authority.");
    assert_eq!(value["entries"][0]["evidence"], "Observed evidence.");
    assert_eq!(value["entries"][0]["exit"], "Remove with the law.");
}

#[test]
fn markdown() {
    let held = Entry::parse(
        "# plumb.seat-missing\n\n## Trigger\n\nA seat is missing.\n\n## Solution\n\nDeclare it.\n\n## Evidence\n\nObserved.\n\n## EXIT\n\nRemove with the law.\n",
    )
    .expect("entry");
    assert_eq!(held.code().text(), "plumb.seat-missing");
    assert_eq!(held.trigger(), "A seat is missing.");
    assert_eq!(held.solution(), "Declare it.");
    assert_eq!(held.evidence(), Some("Observed."));
    assert_eq!(held.exit(), Some("Remove with the law."));
    assert!(matches!(
        Entry::parse("# plumb.seat-missing\n\n## Move\n\nDeclare it."),
        Err(Error::Format(_))
    ));
}
