use std::collections::BTreeSet;
use std::path::Path;

#[derive(Default)]
pub struct Evidence {
    actual: BTreeSet<String>,
}

impl Evidence {
    pub fn names(&self) -> BTreeSet<String> {
        self.actual
            .iter()
            .filter_map(|path| {
                path.strip_prefix(".forgejo/workflows/")
                    .and_then(|name| name.strip_suffix(".yml"))
                    .map(str::to_string)
            })
            .collect()
    }
}

pub fn read(root: &Path) -> Evidence {
    let mut actual = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root.join(".forgejo/workflows")) else {
        return Evidence::default();
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".yml") {
            continue;
        }
        let path = format!(".forgejo/workflows/{name}");
        actual.insert(path);
    }
    Evidence { actual }
}
