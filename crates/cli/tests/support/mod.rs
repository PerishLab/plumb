use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, sha};

#[path = "skeleton.rs"]
mod skeleton;
use skeleton::skeleton;

#[path = "bucket.rs"]
mod bucket;
#[allow(unused_imports)]
pub use bucket::Bucket;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MARK: &str = "29990101T000000Z";
const SOURCES: [(&str, &str); 3] = [
    ("assets", "assets"),
    ("cookbook", "cookbook"),
    ("help", "help"),
];

#[allow(dead_code)]
pub fn depot(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("depot fixture");
    stock(&fixture.path().join("configurations"), overrides);
    fixture
}

#[allow(dead_code)]
pub fn rules(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("rules")
                .join(name);
            let body = std::fs::read_to_string(&path).expect("repository rule source");
            (format!("rules/{name}"), body)
        })
        .collect()
}

#[allow(dead_code)]
pub fn plumb() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"));
    for (name, _) in std::env::vars() {
        let ambient = name.starts_with("CARGO_") || name.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&name.as_str()) {
            command.env_remove(name);
        }
    }
    command
}

#[allow(dead_code)]
pub fn seat() -> &'static Path {
    static SEAT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    SEAT.get_or_init(|| depot(&[]).keep())
}

#[allow(dead_code)]
pub fn object(depot: &Path, path: &str) -> String {
    std::fs::read_to_string(depot.join("configurations").join(MARK).join(path))
        .expect("seat object")
}

#[allow(dead_code)]
pub fn home(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("home fixture");
    stock(&fixture.path().join(".plumb/configurations"), overrides);
    fixture
}

#[allow(dead_code)]
pub fn guard(overrides: &[(&str, &str)], target: &str) -> tempfile::TempDir {
    let fixture = depot(overrides);
    let base = fixture.path().join("configurations").join(MARK);
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for file in walk(&base) {
        let path = file.strip_prefix(&base).expect("guard object");
        if path == Path::new(plumb::depot::LEAF) {
            continue;
        }
        let path = path.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(&file).expect("guard body");
        objects.push(Object {
            path: path.clone(),
            sha256: sha(&bytes),
            size: bytes.len() as u64,
        });
        bodies.insert(path, bytes);
    }
    let manifest = plumb::guard::Configuration::new(
        target.into(),
        plumb::guard::Validator {
            version: "v0.0.1".into(),
            release: "a".repeat(64),
            artifact: "b".repeat(64),
        },
        objects,
    )
    .expect("guard configuration");
    manifest
        .install(fixture.path(), &bodies)
        .expect("guard installation");
    fixture
}

pub fn stock(root: &Path, overrides: &[(&str, &str)]) {
    let base = root.join(MARK);
    let mut bodies = skeleton();
    for (source, seat) in SOURCES {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(source);
        for file in walk(&root) {
            let name = file.strip_prefix(&root).expect("relative object");
            let path = format!("{seat}/{}", name.to_string_lossy().replace('\\', "/"));
            bodies.insert(path, std::fs::read(&file).expect("implementation resource"));
        }
    }
    for (path, text) in overrides {
        bodies.insert((*path).into(), text.as_bytes().to_vec());
    }
    let mut objects = Vec::new();
    for (path, bytes) in bodies {
        let target = base.join(&path);
        std::fs::create_dir_all(target.parent().expect("object parent"))
            .expect("object parent seat");
        std::fs::write(target, &bytes).expect("depot object");
        objects.push(Object {
            path,
            sha256: sha(&bytes),
            size: bytes.len() as u64,
        });
    }
    objects.sort();
    let metadata = Metadata {
        version: MARK.to_string(),
        source: "fixture".to_string(),
        channel: "stable".to_string(),
        commit: String::new(),
    };
    let manifest = Manifest {
        schema: Schema {
            format: FORMAT,
            version: concat!("v", env!("CARGO_PKG_VERSION")).to_string(),
        },
        metadata: metadata.clone(),
        objects,
    };
    std::fs::write(
        base.join("plumb.toml"),
        manifest.encode().expect("manifest"),
    )
    .expect("manifest seat");
    std::fs::write(
        root.join("metadata.json"),
        Pointer::new(&metadata, "plumb").encode().expect("pointer"),
    )
    .expect("pointer seat");
}

#[allow(dead_code)]
pub fn policy(path: &str) -> String {
    let name = path.strip_prefix("rules/").unwrap_or(path);
    rules(&[name])[0].1.clone()
}

pub(super) fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).expect("source root") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}
