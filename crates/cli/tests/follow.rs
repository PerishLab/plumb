use plumb_cli::writer;
use runseal::github::records::Author;

fn author(id: u64, login: &str) -> Author {
    Author {
        id,
        login: login.into(),
    }
}

fn check(operator: Author) -> Result<(), String> {
    let owner = author(1, "fixture-writer[bot]");
    writer::check(17, Some(owner), || Ok(operator))
}

#[test]
fn matching() {
    check(author(1, "fixture-writer[bot]")).unwrap();
    check(author(1, "renamed-writer")).unwrap();
}

#[test]
fn foreign() {
    assert_eq!(
        check(author(2, "foreign-operator")).unwrap_err(),
        "Auto #17 has operation writer fixture-writer[bot] (id 1), but this follow runs as foreign-operator (id 2); only the writer settles its automation-owned Auto, so nothing was written; rerun follow as fixture-writer[bot]"
    );
    let error = check(author(2, "fixture-writer[bot]")).unwrap_err();
    assert!(error.contains("(id 1), but this follow runs as fixture-writer[bot] (id 2)"));
}

#[test]
fn owner() {
    let viewer = || panic!("an unqualified owner must stop before the viewer lookup");
    for owner in [None, Some(author(0, "fixture-writer[bot]"))] {
        assert_eq!(
            writer::check(17, owner, viewer).unwrap_err(),
            "Auto issue has no operation writer identity"
        );
    }
}

#[test]
fn missing() {
    assert_eq!(
        check(author(0, "fixture-writer[bot]")).unwrap_err(),
        "provider identity is unreadable"
    );
    assert_eq!(
        check(author(1, "")).unwrap_err(),
        "provider identity is unreadable"
    );
}

#[test]
fn transport() {
    let owner = author(1, "fixture-writer[bot]");
    let error = writer::check(17, Some(owner), || Err("provider unavailable".into())).unwrap_err();
    assert_eq!(error, "provider unavailable");
}
