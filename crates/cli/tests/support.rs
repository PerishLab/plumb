use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, sha};
use std::path::{Path, PathBuf};

const MARK: &str = "29990101T000000Z";
const RULES: [&str; 9] = [
    "catalog.toml",
    "deps.toml",
    "policy.toml",
    "release.toml",
    "seat.toml",
    "structure.toml",
    "taxonomy.toml",
    "vocabulary.toml",
    "workflow.toml",
];

pub fn depot(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("depot fixture");
    let base = fixture.path().join(MARK);
    let rules = base.join("rules");
    std::fs::create_dir_all(&rules).expect("rules seat");
    let mut objects = Vec::new();
    for name in RULES {
        let path = format!("rules/{name}");
        let bytes = overrides
            .iter()
            .find(|(held, _)| *held == path)
            .map(|(_, text)| text.as_bytes().to_vec())
            .unwrap_or_else(|| std::fs::read(source(name)).expect("rule source"));
        std::fs::write(base.join(&path), &bytes).expect("rule object");
        objects.push(Object {
            path,
            sha256: sha(&bytes),
            size: bytes.len() as u64,
        });
    }
    let metadata = Metadata {
        version: MARK.to_string(),
        source: "fixture".to_string(),
        channel: "stable".to_string(),
        commit: String::new(),
    };
    let manifest = Manifest {
        schema: Schema {
            format: FORMAT,
            version: "v0.0.1".to_string(),
        },
        metadata: metadata.clone(),
        objects,
    };
    std::fs::write(base.join("plumb.toml"), manifest.encode().expect("manifest"))
        .expect("manifest seat");
    std::fs::write(
        fixture.path().join("metadata.json"),
        Pointer::new(&metadata, "plumb").encode().expect("pointer"),
    )
    .expect("pointer seat");
    fixture
}

fn source(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if name == "vocabulary.toml" {
        return root.join("../lib/rules").join(name);
    }
    root.join("rules").join(name)
}
