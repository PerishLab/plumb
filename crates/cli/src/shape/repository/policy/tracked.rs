use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct Tracked(Vec<String>);

impl Tracked {
    pub(super) fn list(root: &Path) -> Self {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(root)
            .args(["ls-files", "-z"])
            .output();
        let paths = match output {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };
        Self(paths)
    }

    pub(super) fn holds(&self, seats: &BTreeSet<String>, suffixes: &BTreeSet<String>) -> bool {
        self.0.iter().any(|path| {
            let seated = seats
                .iter()
                .any(|seat| path.starts_with(&format!("{seat}/")));
            let suffix = Path::new(path)
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| suffixes.contains(value));
            seated && suffix
        })
    }
}

pub(super) fn earning(table: &toml::Table) -> BTreeSet<String> {
    super::list(table, "js")
        .iter()
        .flat_map(|held| {
            let set = crate::catalog::member::scan(held)
                .unwrap_or_else(|error| panic!("rules/suites/shape.toml names {held}: {error}"));
            suffixes(&set)
        })
        .collect()
}

pub(super) fn suffixes(set: &toml::Table) -> BTreeSet<String> {
    super::list(set, "include")
        .iter()
        .filter_map(|glob| glob.rsplit_once("*.").map(|(_, tail)| tail.to_string()))
        .collect()
}
