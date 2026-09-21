use plumb::snapshot::Snapshot;
use std::collections::BTreeMap;

pub use plumb::depot::{Object, POINTER, sha};

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

pub type Held = (Vec<Object>, BTreeMap<String, Vec<u8>>);

pub fn inventory(snapshot: &Snapshot) -> Result<Held, String> {
    gather(snapshot, configuration(snapshot.root())?)
}

fn gather(snapshot: &Snapshot, roots: Vec<(String, String)>) -> Result<Held, String> {
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for (root, seat) in roots {
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

pub fn configuration(root: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    roots(crate::shape::layout::stated(root))
}

fn roots(held: crate::shape::layout::Held) -> Result<Vec<(String, String)>, String> {
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
