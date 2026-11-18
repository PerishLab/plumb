use super::{public, wire};
use plumb::instance::{Declared, Inputs, Instance};
use radicula_config::Cascade;
use std::path::{Path, PathBuf};

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("ssh_host_ed25519_key.pub"),
            public(&wire(&[7; 32])),
        )
        .unwrap();
        Self { root }
    }

    fn hostkey(&self) -> PathBuf {
        self.root.path().join("ssh_host_ed25519_key.pub")
    }

    fn file(&self) -> PathBuf {
        self.root.path().join("plumb.toml")
    }

    fn write(&self, text: &str) {
        std::fs::write(self.file(), text).unwrap();
    }

    fn run(
        &self,
        argument: Option<&str>,
        environment: Option<&str>,
        file: Option<&Path>,
    ) -> Result<Instance, String> {
        let get = |key: &str| {
            (key == "PLUMB_INSTANCE_ID")
                .then(|| environment.map(str::to_string))
                .flatten()
        };
        plumb::instance::resolve(&Inputs {
            argument,
            file,
            get: &get,
            hostkey: &self.hostkey(),
        })
    }
}

fn kind(instance: &Instance) -> &'static str {
    instance.source.kind
}

#[test]
fn names() {
    assert_eq!(Declared::prefix(), "PLUMB");
    let seen = std::cell::RefCell::new(Vec::new());
    let get = |key: &str| {
        seen.borrow_mut().push(key.to_string());
        None
    };
    let _ = Declared::lookup("PLUMB", &get).unwrap();
    assert_eq!(*seen.borrow(), ["PLUMB_INSTANCE_ID"]);
}

fn expected(argument: bool, environment: bool, filed: bool) -> (&'static str, &'static str) {
    if argument {
        return ("argued", "argument");
    }
    if environment {
        return ("enved", "environment");
    }
    if filed {
        return ("filed", "configuration");
    }
    ("", "ssh-host-key")
}

#[test]
fn precedence() {
    let fixture = Fixture::new();
    fixture.write("instance_id = \"filed\"\n");
    let file = fixture.file();
    for bits in 0..8u8 {
        let (argument, environment, filed) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        let held = fixture
            .run(
                argument.then_some("argued"),
                environment.then_some("enved"),
                filed.then_some(file.as_path()),
            )
            .unwrap();
        let (id, source) = expected(argument, environment, filed);
        assert_eq!(kind(&held), source);
        if id.is_empty() {
            assert_eq!(
                held.source.path.as_deref(),
                Some(fixture.hostkey().as_path())
            );
            assert!(held.id.starts_with("ssh-ed25519:SHA256:"));
        } else {
            assert_eq!(held.id, id);
        }
    }
}

#[test]
fn schema() {
    let fixture = Fixture::new();
    fixture.write("instance_id = \"filed\"\n");
    let file = fixture.file();
    let argument = serde_json::to_value(fixture.run(Some("a"), None, None).unwrap()).unwrap();
    assert_eq!(argument["schema"], "plumb.instance/v1");
    assert_eq!(argument["source"]["kind"], "argument");
    assert!(argument["source"].get("path").is_none());
    let environment = serde_json::to_value(fixture.run(None, Some("e"), None).unwrap()).unwrap();
    assert_eq!(environment["source"]["kind"], "environment");
    assert!(environment["source"].get("path").is_none());
    let filed = serde_json::to_value(fixture.run(None, None, Some(&file)).unwrap()).unwrap();
    assert_eq!(filed["source"]["kind"], "configuration");
    assert_eq!(filed["source"]["path"], file.to_str().unwrap());
    let hostkey = serde_json::to_value(fixture.run(None, None, None).unwrap()).unwrap();
    assert_eq!(hostkey["source"]["kind"], "ssh-host-key");
    assert_eq!(
        hostkey["source"]["path"],
        fixture.hostkey().to_str().unwrap()
    );
}

#[test]
fn unread() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.hostkey()).unwrap();
    assert_eq!(fixture.run(Some("a"), None, None).unwrap().id, "a");
    assert_eq!(fixture.run(None, Some("e"), None).unwrap().id, "e");
    fixture.write("instance_id = \"f\"\n");
    assert_eq!(
        fixture.run(None, None, Some(&fixture.file())).unwrap().id,
        "f"
    );
    assert!(fixture.run(None, None, None).is_err());
}

#[test]
fn opaque() {
    let fixture = Fixture::new();
    let widest = "x".repeat(512);
    for value in [
        "plain",
        "ssh-ed25519:SHA256:b3KGuK8CLQYPfetXS7oFjxAbSZdnf6dH9Gw8Ji8jg7Q",
        "registered:future source",
        "实例-一",
        widest.as_str(),
    ] {
        assert_eq!(fixture.run(Some(value), None, None).unwrap().id, value);
        assert_eq!(fixture.run(None, Some(value), None).unwrap().id, value);
        fixture.write(&format!("instance_id = {}\n", quoted(value)));
        let held = fixture.run(None, None, Some(&fixture.file())).unwrap();
        assert_eq!((held.id.as_str(), kind(&held)), (value, "configuration"));
    }
}

fn quoted(value: &str) -> String {
    let mut text = String::from("\"");
    for char in value.chars() {
        match char {
            '"' | '\\' => {
                text.push('\\');
                text.push(char);
            }
            _ if char.is_control() => text.push_str(&format!("\\u{:04X}", char as u32)),
            _ => text.push(char),
        }
    }
    text.push('"');
    text
}

#[test]
fn invalid() {
    let fixture = Fixture::new();
    let long = "x".repeat(513);
    let cases = [
        ("", "empty"),
        (" x", "whitespace"),
        ("x ", "whitespace"),
        ("\tx", "whitespace"),
        ("x\n", "whitespace"),
        ("a\nb", "control"),
        ("a\tb", "control"),
        ("a\0b", "control"),
        ("a\u{7f}b", "control"),
        (long.as_str(), "at most 512"),
    ];
    for (value, rule) in cases {
        let error = fixture.run(Some(value), None, None).unwrap_err();
        assert!(error.contains("--instance-id"), "{error}");
        assert!(error.contains(rule), "{error}");
        fixture.write(&format!("instance_id = {}\n", quoted(value)));
        let error = fixture.run(None, None, Some(&fixture.file())).unwrap_err();
        assert!(error.contains("plumb.toml"), "{error}");
        assert!(error.contains(rule), "{error}");
    }
    for value in ["a\nb", "a\u{1}b", long.as_str()] {
        let error = fixture.run(None, Some(value), None).unwrap_err();
        assert!(error.contains("PLUMB_INSTANCE_ID"), "{error}");
    }
}

#[test]
fn blank() {
    let fixture = Fixture::new();
    fixture.write("instance_id = \"filed\"\n");
    let file = fixture.file();
    for value in ["", " ", "\t \n"] {
        let held = fixture.run(None, Some(value), Some(&file)).unwrap();
        assert_eq!((held.id.as_str(), kind(&held)), ("filed", "configuration"));
        let held = fixture.run(None, Some(value), None).unwrap();
        assert_eq!(kind(&held), "ssh-host-key");
    }
    let held = fixture.run(None, Some("  padded \n"), None).unwrap();
    assert_eq!((held.id.as_str(), kind(&held)), ("padded", "environment"));
}

#[test]
fn unfallen() {
    let fixture = Fixture::new();
    fixture.write("instance_id = \"filed\"\n");
    let file = fixture.file();
    assert!(fixture.run(Some(""), Some("enved"), Some(&file)).is_err());
    assert!(fixture.run(Some(" a"), None, Some(&file)).is_err());
    assert!(fixture.run(None, Some("a\u{1}"), Some(&file)).is_err());
    fixture.write("instance_id = \"\"\n");
    assert!(fixture.run(None, None, Some(&file)).is_err());
    let shadowed = fixture.run(None, Some("enved"), Some(&file)).unwrap();
    assert_eq!(
        (shadowed.id.as_str(), kind(&shadowed)),
        ("enved", "environment")
    );
}

#[test]
fn filed() {
    let fixture = Fixture::new();
    let file = fixture.file();
    let held = fixture.run(None, None, Some(&file)).unwrap();
    assert_eq!(kind(&held), "ssh-host-key");
    fixture.write(
        "instance_id = \"filed\"\nunknown = 1\n[locus]\nenabled = true\n[other]\nkey = \"value\"\n",
    );
    let held = fixture.run(None, None, Some(&file)).unwrap();
    assert_eq!((held.id.as_str(), kind(&held)), ("filed", "configuration"));
    assert_eq!(held.source.path.as_deref(), Some(file.as_path()));
    fixture.write("[locus]\nenabled = true\ninstance_id = \"misplaced\"\n");
    let held = fixture.run(None, None, Some(&file)).unwrap();
    assert_eq!(kind(&held), "ssh-host-key");
    fixture.write("[locus]\nenabled = true\n");
    assert_eq!(
        kind(&fixture.run(None, None, Some(&file)).unwrap()),
        "ssh-host-key"
    );
}

#[test]
fn corrupt() {
    let fixture = Fixture::new();
    let file = fixture.file();
    for text in [
        "instance_id = ",
        "instance_id = 5\n",
        "instance_id = [\"a\"]\n",
        "[[\n",
    ] {
        fixture.write(text);
        for argument in [None, Some("argued")] {
            let error = fixture.run(argument, None, Some(&file)).unwrap_err();
            assert!(error.contains("plumb.toml"), "{error}");
        }
    }
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(fixture.run(None, None, Some(&file)).is_err());
}
