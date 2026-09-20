use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, sha};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod bucket;

#[allow(unused_imports)]
pub use bucket::Bucket;

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

const TAXONOMY: &str = "[[owner]]\nid = \"fixture\"\nsummary = \"fixture\"\n\n[[tag]]\nid = \"fixture\"\nsummary = \"fixture\"\n";
const POLICY: &str = "[limit]\n\n[comment]\nallow = false\n\n[word]\nsingle = true\n\n[[shape]]\nwhen = [\"fixture-absent\"]\n\n[[web]]\nseat = \"fixture-absent\"\n";
const STRUCTURE: &str = "[dir]\n\n[lane]\n";
const WORKFLOW: &str = "[suite]\n";
const DEPS: &str = "blacklist = []\n\n[stable.cargo]\nregistry = \"fixture\"\nindex = \"sparse+https://registry.invalid/\"\n";
const RELEASE: &str =
    "ceiling = 1000\n\n[forge]\nimage = \"fixture\"\n\n[permitted]\n\n[exercised]\n";
const SEAT: &str = "[member]\n";
const VOCABULARY: &str = "schema = 1\ncodec = \"p64-v1\"\nretired = []\n";

fn skeleton() -> BTreeMap<String, Vec<u8>> {
    let ids = mechanisms();
    let mut namespaces: Vec<&str> = ids
        .iter()
        .map(|id| id.split('.').next().expect("namespaced mechanism"))
        .collect();
    namespaces.dedup();
    let mut words = TAXONOMY.to_string();
    for namespace in namespaces {
        words.push_str(&format!(
            "\n[[namespace]]\nid = \"{namespace}\"\nsummary = \"fixture\"\nowner = \"fixture\"\n"
        ));
    }
    let mut law = String::new();
    for id in &ids {
        law.push_str(&format!(
            "[[rule]]\nid = \"{id}\"\nsummary = \"fixture\"\nlaw = \"fixture\"\nevidence = \"fixture\"\nstanding = \"mechanized\"\nowner = \"fixture\"\ntags = [\"fixture\"]\n\n"
        ));
    }
    [
        ("rules/taxonomy.toml", words),
        ("rules/catalog.toml", law),
        ("rules/policy.toml", POLICY.to_string()),
        ("rules/structure.toml", STRUCTURE.to_string()),
        ("rules/workflow.toml", WORKFLOW.to_string()),
        ("rules/deps.toml", DEPS.to_string()),
        ("rules/release.toml", RELEASE.to_string()),
        ("rules/seat.toml", SEAT.to_string()),
        ("rules/vocabulary.toml", VOCABULARY.to_string()),
    ]
    .into_iter()
    .map(|(path, text)| (path.to_string(), text.into_bytes()))
    .collect()
}

fn mechanisms() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/catalog/rules");
    let mut ids = Vec::new();
    for file in walk(&root) {
        let text = std::fs::read_to_string(&file).expect("mechanism source");
        for call in text.split("rule!(").skip(1) {
            let args = call.split(')').next().unwrap_or_default();
            if let Some(id) = args.split('"').nth(1) {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

#[allow(dead_code)]
fn walk(root: &Path) -> Vec<PathBuf> {
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
