use std::collections::BTreeSet;
use std::path::Path;

const GUARDS: [&str; 2] = [
    ".forgejo/workflows/guard.yml",
    ".github/workflows/quality.yml",
];

pub struct Guard {
    pub lanes: Vec<(String, String)>,
    pub source: String,
}

pub struct Operator<'a>(pub &'a Path);

impl Operator<'_> {
    pub fn guard(&self) -> Guard {
        let mut lanes = Vec::new();
        for seat in GUARDS {
            let Ok(source) = std::fs::read_to_string(self.0.join(seat)) else {
                continue;
            };
            lanes.push((seat.to_string(), source.replace("\r\n", "\n")));
        }
        let mut sources = vec![
            std::fs::read_to_string(self.0.join(".runseal/wrappers/guard.ts")).unwrap_or_default(),
        ];
        sources.extend(lanes.iter().map(|(_, source)| source.clone()));
        Guard {
            lanes,
            source: sources.join("\n"),
        }
    }

    pub fn actions(&self) -> BTreeSet<String> {
        let Ok(entries) = std::fs::read_dir(self.0) else {
            return BTreeSet::new();
        };
        entries
            .flatten()
            .filter(|entry| {
                entry.path().is_dir()
                    && (entry.path().join("action.yml").is_file()
                        || entry.path().join("action.yaml").is_file())
            })
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect()
    }

    pub fn tests(&self) -> Vec<String> {
        let mut found = Vec::new();
        collect(self.0, &self.0.join(".runseal"), &mut found);
        found.sort();
        found
    }
}

fn collect(root: &Path, at: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, found);
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.ends_with(".test.ts")
            || name.ends_with("_test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with("_test.tsx"))
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_string_lossy().to_string());
        }
    }
}
