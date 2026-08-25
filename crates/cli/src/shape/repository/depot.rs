use plumb::snapshot::Snapshot;
use std::collections::BTreeMap;

pub use plumb::depot::{FORMAT, LEAF, Manifest, Object, POINTER, Pointer, latest, sha, versions};

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
    pub fn configuration(snapshot: &Snapshot, draft: Draft) -> Result<Self, String> {
        let (objects, bodies) = inventory(snapshot)?;
        for (root, _) in configuration(snapshot.root())? {
            if snapshot.seat(&root).is_empty() {
                return Err(format!("depot root {root} records no object"));
            }
        }
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

    pub fn changelog(draft: Draft, bodies: BTreeMap<String, Vec<u8>>) -> Result<Self, String> {
        Self::gather(plumb::depot::v2::Kind::Changelog, draft, bodies)
    }

    fn gather(
        derivative: plumb::depot::v2::Kind,
        draft: Draft,
        bodies: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, String> {
        if bodies.is_empty() {
            return Err(format!("{} derivative holds no object", derivative.label()));
        }
        let objects = bodies
            .iter()
            .map(|(path, bytes)| Object {
                path: path.clone(),
                sha256: sha(bytes),
                size: bytes.len() as u64,
            })
            .collect();
        let manifest = plumb::depot::v2::Manifest {
            format: plumb::depot::v2::FORMAT,
            source: draft.source,
            derivative,
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
