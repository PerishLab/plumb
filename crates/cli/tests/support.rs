use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, sha};
use std::path::{Path, PathBuf};

const MARK: &str = "29990101T000000Z";
const SOURCES: [(&str, &str); 5] = [
    ("rules", "rules"),
    ("../lib/rules", "rules"),
    ("assets", "assets"),
    ("cookbook", "cookbook"),
    ("help", "help"),
];

#[allow(dead_code)]
pub fn depot(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("depot fixture");
    stock(&fixture.path().join("depot"), overrides);
    fixture
}

pub fn stock(root: &Path, overrides: &[(&str, &str)]) {
    let base = root.join(MARK);
    let mut objects = Vec::new();
    for (source, seat) in SOURCES {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(source);
        for file in walk(&root) {
            let name = file.strip_prefix(&root).expect("relative object");
            let path = format!("{seat}/{}", name.to_string_lossy().replace('\\', "/"));
            let bytes = overrides
                .iter()
                .find(|(held, _)| *held == path)
                .map(|(_, text)| text.as_bytes().to_vec())
                .unwrap_or_else(|| std::fs::read(&file).expect("object source"));
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
