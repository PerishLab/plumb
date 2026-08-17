use plumb::fill::{Error, fill};
use std::collections::BTreeMap;

fn table() -> BTreeMap<&'static str, String> {
    BTreeMap::from([("port", "3500".to_string()), ("name", "api".to_string())])
}

#[test]
fn filled() {
    let out = fill("http://127.0.0.1:{port}/health", &table()).expect("fill");
    assert_eq!(out, "http://127.0.0.1:3500/health");
    let two = fill("{name}:{port}", &table()).expect("fill");
    assert_eq!(two, "api:3500");
}

#[test]
fn plain() {
    assert_eq!(fill("no vars", &table()).expect("fill"), "no vars");
    assert_eq!(fill("", &table()).expect("fill"), "");
}

#[test]
fn braced() {
    assert_eq!(fill("{{port}}", &table()).expect("fill"), "{port}");
    assert_eq!(fill("a{{b}}c", &table()).expect("fill"), "a{b}c");
}

#[test]
fn refused() {
    assert_eq!(
        fill("{ghost}", &table()),
        Err(Error::Unknown {
            name: "ghost".into()
        })
    );
    assert_eq!(fill("{port", &table()), Err(Error::Unclosed));
    assert_eq!(fill("port}", &table()), Err(Error::Bare));
    assert_eq!(fill("{}", &table()), Err(Error::Empty));
}

#[test]
fn expressions() {
    let vars = BTreeMap::from([("product", "plumb".to_string())]);
    let text = "image: forge\nrun: ${{ matrix.medium }} {@product}\njq: '{context: $held}'\n";
    assert_eq!(
        plumb::fill::actions(text, &vars).expect("render"),
        "image: forge\nrun: ${{ matrix.medium }} plumb\njq: '{context: $held}'\n"
    );
}

#[test]
fn unknown() {
    let vars = BTreeMap::from([("product", "plumb".to_string())]);
    assert_eq!(
        plumb::fill::actions("{@absent}", &vars),
        Err(plumb::fill::Error::Unknown {
            name: "absent".to_string()
        })
    );
    assert_eq!(
        plumb::fill::actions("{@open", &vars),
        Err(plumb::fill::Error::Unclosed)
    );
}
