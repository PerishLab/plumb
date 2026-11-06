use plumb::semantics::subjects::{Role, Subjects};
use serde_json::{Value, json};

fn declaration() -> Value {
    json!({"schema":Subjects::SCHEMA,"product":"seatrial","cases":[{
        "id":"writer", "role":"normative", "subjects":["plumb"],
        "fixtures":["runseal"], "preparation":["ectropy"],
        "conditions":[{"platform":"linux-x86_64","backend":"none"}],
        "evidence":["writer/main.py"]
    }]})
}

#[test]
fn ownership() {
    let mut held = Subjects::parse(&serde_json::to_vec(&declaration()).unwrap()).unwrap();
    held.bind(&["writer".into()], &["writer/main.py".into()])
        .unwrap();
    assert_eq!(held.coverage("plumb").unwrap().len(), 1);
    assert!(held.coverage("runseal").unwrap().is_empty());
    assert!(held.coverage("ectropy").unwrap().is_empty());
    held.cases[0].role = Role::Diagnostic;
    assert!(held.coverage("plumb").unwrap().is_empty());
}

#[test]
fn completeness() {
    let held = Subjects::parse(&serde_json::to_vec(&declaration()).unwrap()).unwrap();
    let paths = ["writer/main.py".into()];
    assert!(held.bind(&[], &paths).is_err());
    assert!(
        held.bind(&["writer".into(), "hidden".into()], &paths)
            .is_err()
    );
    assert!(
        held.bind(&["writer".into(), "writer".into()], &paths)
            .is_err()
    );
    assert!(held.bind(&["writer".into()], &[]).is_err());
}

#[test]
fn strictness() {
    for field in ["schema", "product", "cases"] {
        let mut held = declaration();
        held.as_object_mut().unwrap().remove(field);
        assert!(Subjects::parse(&serde_json::to_vec(&held).unwrap()).is_err());
    }
    for field in [
        "id",
        "role",
        "subjects",
        "fixtures",
        "preparation",
        "conditions",
        "evidence",
    ] {
        let mut held = declaration();
        held["cases"][0].as_object_mut().unwrap().remove(field);
        assert!(Subjects::parse(&serde_json::to_vec(&held).unwrap()).is_err());
    }
    let mut held = declaration();
    held["cases"][0]["qualification"] = json!(true);
    assert!(Subjects::parse(&serde_json::to_vec(&held).unwrap()).is_err());
    let raw = serde_json::to_string(&declaration()).unwrap().replace(
        "\"product\":\"seatrial\"",
        "\"product\":\"seatrial\",\"product\":\"plumb\"",
    );
    assert!(Subjects::parse(raw.as_bytes()).is_err());
}

#[test]
fn identities() {
    for (field, value) in [
        ("evidence", json!(["../main.py"])),
        ("evidence", json!(["writer/main.py", "writer/main.py"])),
        ("subjects", json!([])),
        ("subjects", json!(["plumb", "plumb"])),
        ("fixtures", json!(["PerishLab/runseal"])),
        ("conditions", json!([])),
        ("role", json!("selected")),
    ] {
        let mut held = declaration();
        held["cases"][0][field] = value;
        assert!(
            Subjects::parse(&serde_json::to_vec(&held).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut held = declaration();
    let duplicate = held["cases"][0].clone();
    held["cases"].as_array_mut().unwrap().push(duplicate);
    assert!(Subjects::parse(&serde_json::to_vec(&held).unwrap()).is_err());
}
