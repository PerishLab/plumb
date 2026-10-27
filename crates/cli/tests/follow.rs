use std::cell::RefCell;

use plumb_cli::writer;
use serde_json::{Value, json};

struct Mock {
    owner: Value,
    viewer: Value,
    calls: RefCell<Vec<Vec<String>>>,
}

impl Mock {
    fn new(owner: Value, viewer: Value) -> Self {
        Self {
            owner,
            viewer,
            calls: RefCell::default(),
        }
    }

    fn command(&self, args: &[String]) -> Result<Vec<u8>, String> {
        self.calls.borrow_mut().push(args.to_vec());
        let value = match args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["api", "repos/Example/probe/issues/17"] => json!({"user": self.owner}),
            [
                "api",
                "graphql",
                "-f",
                "query=query{viewer{databaseId login}}",
            ] => json!({"data": {"viewer": self.viewer}}),
            _ => panic!("unexpected provider operation: {args:?}"),
        };
        Ok(serde_json::to_vec(&value).unwrap())
    }

    fn check(&self) -> Result<(), String> {
        writer::check("Example/probe", 17, |args| self.command(args))
    }
}

fn fixture(viewer: Value) -> Mock {
    Mock::new(json!({"id": 1, "login": "fixture-writer[bot]"}), viewer)
}

#[test]
fn matching() {
    let mock = fixture(json!({"databaseId": 1, "login": "fixture-writer[bot]"}));
    mock.check().unwrap();
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn renamed() {
    let mock = fixture(json!({"databaseId": 1, "login": "renamed-writer"}));
    mock.check().unwrap();
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn foreign() {
    let mock = fixture(json!({"databaseId": 2, "login": "foreign-operator"}));
    assert_eq!(
        mock.check().unwrap_err(),
        "Auto #17 has operation writer fixture-writer[bot] (id 1), but this follow runs as foreign-operator (id 2); only the writer settles its automation-owned Auto, so nothing was written; rerun follow as fixture-writer[bot]"
    );
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn impersonated() {
    let mock = fixture(json!({"databaseId": 2, "login": "fixture-writer[bot]"}));
    let error = mock.check().unwrap_err();
    assert!(error.contains("(id 1), but this follow runs as fixture-writer[bot] (id 2)"));
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn owner() {
    let mock = Mock::new(json!({"login": "fixture-writer[bot]"}), Value::Null);
    assert_eq!(
        mock.check().unwrap_err(),
        "Auto issue has no operation writer identity"
    );
    assert_eq!(mock.calls.borrow().len(), 1);
}

#[test]
fn missing() {
    let mock = fixture(json!({"login": "fixture-writer[bot]"}));
    assert_eq!(mock.check().unwrap_err(), "provider identity is unreadable");
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn absent() {
    let mock = fixture(Value::Null);
    assert_eq!(mock.check().unwrap_err(), "provider identity is unreadable");
    assert_eq!(mock.calls.borrow().len(), 2);
}

#[test]
fn transport() {
    let calls = RefCell::new(Vec::new());
    let error = writer::check("Example/probe", 17, |args| {
        calls.borrow_mut().push(args.to_vec());
        Err("provider unavailable".into())
    })
    .unwrap_err();
    assert_eq!(error, "provider unavailable");
    assert_eq!(
        *calls.borrow(),
        vec![vec!["api", "repos/Example/probe/issues/17"]]
    );
}
