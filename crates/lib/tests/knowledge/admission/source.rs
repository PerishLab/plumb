use super::baseline;
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, write::GzEncoder};
use plumb::guard::Runtime;
use plumb::semantics::admission::{Payload, Record, World};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, process::Command};

struct Inputs {
    world: World,
    files: Vec<Value>,
}

impl Inputs {
    fn git(root: &Path, arguments: &[&str]) -> String {
        let mut command = Command::new("git");
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap());
        for name in ["SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let result = command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", root.join("absent-config"))
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().into()
    }

    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        Self::git(root, &["init", "--initial-branch=main"]);
        std::fs::create_dir(root.join("dir")).unwrap();
        for (path, raw) in [
            (".gitattributes", b"hidden.bin export-ignore\n".as_slice()),
            ("hidden.bin", b"\0\xff\n".as_slice()),
            ("dir/a", b"original public interface\n".as_slice()),
            ("dir/b.sh", b"source data, never executed\n".as_slice()),
        ] {
            std::fs::write(root.join(path), raw).unwrap();
        }
        Self::git(root, &["add", "."]);
        Self::git(root, &["update-index", "--chmod=+x", "dir/b.sh"]);
        let tree = Self::git(root, &["write-tree"]);
        let files = Self::git(root, &["ls-files", "--stage"]).lines().map(|line| {
            let (fields, path) = line.split_once('\t').unwrap();
            let fields = fields.split_whitespace().collect::<Vec<_>>();
            let raw = std::fs::read(root.join(path)).unwrap();
            json!({"path":path,"mode":fields[0],"oid":fields[1],"size":raw.len(),"sha256":Self::sha(&raw),"data":STANDARD.encode(raw)})
        }).collect();
        let (fixture, reference) = baseline::setup();
        let Record::Adoption(adoption) = Record::parse(&fixture.values[&reference.url]).unwrap()
        else {
            panic!("adoption fixture");
        };
        let mut world = World::parse(&fixture.values[&adoption.world.url]).unwrap();
        world.tree = tree.clone();
        world.guard.guard.tree = tree.clone();
        world.guard.guard = world
            .guard
            .guard
            .clone()
            .resolved(plumb::packages::Resolution {
                context: "ci-latest".into(),
                tree,
                packages: world.packages.clone(),
            })
            .unwrap();
        world.guard.digest = Runtime::seal(&world.guard.commit, &world.guard.guard).unwrap();
        world.validate().unwrap();
        Self { world, files }
    }

    fn sha(raw: &[u8]) -> String {
        format!("{:x}", Sha256::digest(raw))
    }

    fn packed(raw: &[u8]) -> Value {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(raw).unwrap();
        json!({"encoding":"gzip+base64","size":raw.len(),"sha256":Self::sha(raw),"data":STANDARD.encode(gzip.finish().unwrap())})
    }

    fn value(&self) -> Value {
        json!({"schema":"wharf.release.source/v1","source":self.world.source,"head":self.world.head,"tree":self.world.tree,
            "files":self.files.len(),"payload":Self::packed(&serde_json::to_vec(&self.files).unwrap())})
    }

    fn parse(&self, value: &Value) -> Result<Payload, plumb::windtun::qualification::Fault> {
        Payload::parse(&serde_json::to_vec(value).unwrap(), &self.world)
    }
}

#[test]
fn complete() {
    let held = Inputs::new();
    let raw = serde_json::to_vec(&held.value()).unwrap();
    let observed = Payload::parse(&raw, &held.world).unwrap();
    assert_eq!(observed.sha256, Inputs::sha(&raw));
    assert_ne!(observed.source.tree, observed.tree);
    assert_eq!(observed.files.len(), 4);
    let binary = observed
        .files
        .iter()
        .find(|file| file.path == "hidden.bin")
        .unwrap();
    assert_eq!(STANDARD.decode(&binary.data).unwrap(), b"\0\xff\n");
    assert_eq!(
        observed
            .files
            .iter()
            .find(|file| file.path == "dir/b.sh")
            .unwrap()
            .mode,
        "100755"
    );
}

#[test]
fn binding() {
    let held = Inputs::new();
    for (field, value) in [
        ("tree", json!("0".repeat(40))),
        ("head", json!("1".repeat(40))),
        ("source", json!({})),
        ("files", json!(5)),
        ("files", json!(0)),
        ("files", json!(4097)),
        ("schema", json!("foreign")),
        ("authentication", json!(true)),
    ] {
        let mut changed = held.value();
        changed[field] = value;
        assert!(held.parse(&changed).is_err(), "{field}");
    }
}

#[test]
fn inventory() {
    let mut held = Inputs::new();
    let original = held.files.clone();
    for (field, value) in [
        ("data", json!(STANDARD.encode(b"different"))),
        ("oid", json!("0".repeat(40))),
        ("sha256", json!("0".repeat(64))),
        ("size", json!(99)),
        ("mode", json!("120000")),
        ("path", json!("../escape")),
        ("path", json!("/absolute")),
        ("path", json!("dir\\alias")),
        ("path", json!("dir/./alias")),
        ("path", json!("C:/drive")),
        ("unknown", json!(true)),
    ] {
        held.files = original.clone();
        held.files[0][field] = value;
        assert!(held.parse(&held.value()).is_err(), "{field}");
    }
    held.files = original.clone();
    held.files.pop();
    assert!(
        held.parse(&held.value())
            .unwrap_err()
            .reason
            .contains("Git tree differs")
    );
    held.files = original.clone();
    held.files.push(original[0].clone());
    assert!(held.parse(&held.value()).is_err());
    held.files = original;
    held.files[0]["path"] = json!("dir");
    assert!(held.parse(&held.value()).is_err());
    held.files[0]["path"] = json!("目录/file");
    assert_eq!(
        held.parse(&held.value()).unwrap_err().state,
        plumb::windtun::qualification::State::Unknown
    );
}

#[test]
fn compression() {
    let held = Inputs::new();
    for (field, value) in [
        ("data", json!("bad")),
        ("encoding", json!("base64")),
        ("size", json!(67108865)),
        ("size", json!(1)),
        ("sha256", json!("0".repeat(64))),
        ("extra", json!(true)),
    ] {
        let mut changed = held.value();
        changed["payload"][field] = value;
        assert!(held.parse(&changed).is_err(), "{field}");
    }
    let mut changed = held.value();
    let mut gzip = STANDARD
        .decode(changed["payload"]["data"].as_str().unwrap())
        .unwrap();
    gzip.extend_from_within(..);
    changed["payload"]["data"] = json!(STANDARD.encode(gzip));
    assert!(
        held.parse(&changed)
            .unwrap_err()
            .reason
            .contains("trailing or concatenated")
    );
    let raw = serde_json::to_string(&held.files).unwrap().replacen(
        "\"path\":",
        "\"path\":\"duplicate\",\"path\":",
        1,
    );
    changed["payload"] = Inputs::packed(raw.as_bytes());
    assert!(
        held.parse(&changed)
            .unwrap_err()
            .reason
            .contains("duplicate")
    );
}

#[test]
fn directories() {
    let mut held = Inputs::new();
    let original = held.files[0].clone();
    held.files = (0..4096)
        .map(|number| {
            let mut file = original.clone();
            file["path"] = json!(format!("{number}/a/b/c/d/e/file"));
            file
        })
        .collect();
    assert!(
        held.parse(&held.value())
            .unwrap_err()
            .reason
            .contains("directory inventory exceeds bounds")
    );
}
