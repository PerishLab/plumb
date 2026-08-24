mod guard;

pub(super) use guard::Operator;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone)]
enum Read {
    Unread,
    Held(String),
}

#[derive(Default)]
pub struct Evidence {
    actual: BTreeMap<String, Read>,
    projected: Vec<Projection>,
}

pub struct Projection {
    pub path: String,
    pub rendered: String,
    found: Option<Read>,
}

impl Projection {
    pub fn drifted(&self) -> bool {
        !matches!(&self.found, Some(Read::Held(found)) if found == &self.rendered)
    }

    pub fn absent(&self) -> bool {
        !matches!(self.found, Some(Read::Held(_)))
    }
}

impl Evidence {
    pub fn names(&self) -> BTreeSet<String> {
        self.actual
            .keys()
            .filter_map(|path| {
                path.strip_prefix(".forgejo/workflows/")
                    .and_then(|name| name.strip_suffix(".yml"))
                    .map(str::to_string)
            })
            .collect()
    }

    pub fn project(&mut self, expected: impl IntoIterator<Item = (String, String)>) {
        self.projected = expected
            .into_iter()
            .map(|(path, rendered)| Projection {
                found: self.actual.get(&path).cloned(),
                path,
                rendered,
            })
            .collect();
    }

    pub fn projected(&self) -> &[Projection] {
        &self.projected
    }
}

pub fn read(root: &Path) -> Evidence {
    let mut actual = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(root.join(".forgejo/workflows")) else {
        return Evidence::default();
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".yml") {
            continue;
        }
        let path = format!(".forgejo/workflows/{name}");
        let found = std::fs::read_to_string(entry.path())
            .map(|text| Read::Held(text.replace("\r\n", "\n")))
            .unwrap_or(Read::Unread);
        actual.insert(path, found);
    }
    Evidence {
        actual,
        projected: Vec::new(),
    }
}
