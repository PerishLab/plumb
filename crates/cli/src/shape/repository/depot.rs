use plumb::snapshot::Snapshot;
use std::collections::BTreeMap;

pub use plumb::depot::{
    FORMAT, LEAF, Manifest, Metadata, Object, POINTER, Pointer, Schema, latest, sha, versions,
};

pub(crate) enum Evidence {
    Absent,
    Blind(String),
    Held {
        manifest: Manifest,
        inventory: Option<Result<Vec<Object>, String>>,
    },
}

pub const ROOTS: [(&str, &str); 5] = [
    ("crates/cli/rules", "rules"),
    ("crates/lib/rules", "rules"),
    ("crates/cli/assets", "assets"),
    ("crates/cli/cookbook", "cookbook"),
    ("crates/cli/help", "help"),
];

pub struct Plan {
    pub manifest: Manifest,
    pub bodies: BTreeMap<String, Vec<u8>>,
}

pub type Held = (Vec<Object>, BTreeMap<String, Vec<u8>>);

pub fn inventory(snapshot: &Snapshot) -> Result<Held, String> {
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for (root, seat) in ROOTS {
        for entry in snapshot.seat(root) {
            let name = entry
                .path()
                .strip_prefix(root)
                .map(|rest| rest.trim_start_matches('/'))
                .ok_or_else(|| format!("depot root {root} does not hold {}", entry.path()))?;
            let path = format!("{seat}/{name}");
            if bodies.contains_key(&path) {
                return Err(format!("depot object {path} is claimed twice"));
            }
            objects.push(Object {
                path: path.clone(),
                sha256: sha(entry.bytes()),
                size: entry.bytes().len() as u64,
            });
            bodies.insert(path, entry.bytes().to_vec());
        }
    }
    objects.sort();
    Ok((objects, bodies))
}

impl Plan {
    pub fn gather(snapshot: &Snapshot, metadata: Metadata, schema: Schema) -> Result<Self, String> {
        for (root, _) in ROOTS {
            if snapshot.seat(root).is_empty() {
                return Err(format!("depot root {root} records no object"));
            }
        }
        let (objects, bodies) = inventory(snapshot)?;
        Ok(Self {
            manifest: Manifest {
                schema,
                metadata,
                objects,
            },
            bodies,
        })
    }
}
