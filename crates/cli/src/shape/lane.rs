use std::collections::BTreeSet;
use std::path::Path;

const SEATS: [&str; 2] = [".github/workflows", ".forgejo/workflows"];

#[derive(Default)]
pub struct Evidence {
    actual: BTreeSet<String>,
}

impl Evidence {
    pub fn names(&self) -> BTreeSet<String> {
        self.actual
            .iter()
            .filter_map(|path| name(path))
            .map(str::to_string)
            .collect()
    }
}

pub fn name(path: &str) -> Option<&str> {
    let (seat, file) = path.rsplit_once('/')?;
    if !SEATS.contains(&seat) {
        return None;
    }
    file.strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))
}

pub fn read(root: &Path) -> Evidence {
    let mut actual = BTreeSet::new();
    for seat in SEATS {
        let Ok(entries) = std::fs::read_dir(root.join(seat)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = format!("{seat}/{}", entry.file_name().to_string_lossy());
            if name(&path).is_some() {
                actual.insert(path);
            }
        }
    }
    Evidence { actual }
}
