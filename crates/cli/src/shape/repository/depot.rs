use plumb::snapshot::Snapshot;
use std::collections::BTreeMap;

pub use plumb::depot::{FORMAT, LEAF, Object, POINTER, sha};

pub struct Manifest {
    pub mark: String,
    pub floor: String,
    pub version: Option<String>,
    pub objects: Vec<Object>,
}

pub(crate) enum Evidence {
    Absent,
    Blind(String),
    Held {
        manifest: Manifest,
        inventory: Option<Result<Vec<Object>, String>>,
    },
}

pub struct Batch {
    pub manifest: plumb::depot::v2::Manifest,
    pub bodies: BTreeMap<String, Vec<u8>>,
}

pub struct Draft {
    pub source: String,
    pub release: plumb::depot::v2::Release,
    pub timestamp: String,
    pub commit: String,
}

pub type Held = (Vec<Object>, BTreeMap<String, Vec<u8>>);

pub fn inventory(snapshot: &Snapshot) -> Result<Held, String> {
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for (root, seat) in configuration(snapshot.root())? {
        for entry in snapshot.seat(&root) {
            let name = entry
                .path()
                .strip_prefix(&root)
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

impl Batch {
    pub fn compatibility(bundle: &plumb::depot::v3::Bundle, draft: Draft) -> Result<Self, String> {
        if bundle.manifest.kind != plumb::depot::v3::Kind::Configuration {
            return Err("configuration validation received another depot kind".into());
        }
        let objects = bundle
            .manifest
            .objects
            .iter()
            .map(|held| Object {
                path: held.path.clone(),
                sha256: held.sha256.clone(),
                size: held.size,
            })
            .collect();
        let manifest = plumb::depot::v2::Manifest {
            format: plumb::depot::v2::FORMAT,
            source: draft.source,
            derivative: plumb::depot::v2::Kind::Configuration,
            release: draft.release,
            snapshot: plumb::depot::v2::Snapshot {
                timestamp: draft.timestamp,
                commit: draft.commit,
            },
            objects,
        };
        manifest.encode()?;
        Ok(Self {
            manifest,
            bodies: bundle.bodies.clone(),
        })
    }
    pub fn validation(held: Held, draft: Draft) -> Result<Self, String> {
        let (objects, bodies) = held;
        let manifest = plumb::depot::v2::Manifest {
            format: plumb::depot::v2::FORMAT,
            source: draft.source,
            derivative: plumb::depot::v2::Kind::Configuration,
            release: draft.release,
            snapshot: plumb::depot::v2::Snapshot {
                timestamp: draft.timestamp,
                commit: draft.commit,
            },
            objects,
        };
        manifest.encode()?;
        Ok(Self { manifest, bodies })
    }
}

pub fn configuration(root: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let held = crate::shape::layout::stated(root);
    let crate::shape::layout::Held::Stated(layout) = held else {
        return Err(match held {
            crate::shape::layout::Held::Wrong(error) => error,
            _ => "configuration depot requires a declared layout".to_string(),
        });
    };
    let roots = layout
        .seats
        .into_iter()
        .filter(|seat| !seat.retired)
        .filter_map(|seat| seat.depot.map(|target| (seat.path, target)))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err("layout declares no depot configuration seat".into());
    }
    Ok(roots)
}
