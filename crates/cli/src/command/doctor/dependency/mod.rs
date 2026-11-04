mod registry;

use crate::catalog::set;
use crate::shape::Dependencies;
use plumb::datum::Datum;
use plumb_cli::{Dependency, Verdict};
use semver::Version;
use std::path::Path;

pub fn observe(dependencies: &mut Dependencies, root: &Path, line: Option<&str>) {
    Observer(dependencies).read(root, line);
}

struct Observer<'a>(&'a mut Dependencies);

impl Observer<'_> {
    fn read(&mut self, root: &Path, line: Option<&str>) {
        let observed = registry::observed(root);
        self.0.locked = matches!(
            &observed,
            Some(Ok(held)) if held.resolution.context == "local-locked"
        );
        match line {
            Some(version) => self.recorded(root, version),
            None => registry::read(self.0, root, &observed),
        }
        self.0.normalize();
    }

    fn recorded(&mut self, root: &Path, version: &str) {
        match plumb::datum::Tree(root).read(version) {
            Ok(Some(datum)) => self.against(&datum),
            Ok(None) => self.0.missing = Some(version.to_string()),
            Err(error) => self.0.blind.push(error),
        }
    }

    fn against(&mut self, datum: &Datum) {
        let mut blind = Vec::new();
        for dependency in &mut self.0.held {
            if retired(&dependency.name) {
                continue;
            }
            match datum.latest(dependency.ecosystem.name(), &dependency.name) {
                Some(latest) => dependency.latest = Some(latest.to_string()),
                None => blind.push(format!(
                    "cannot compare {} {}: the {} datum records no answer for it",
                    dependency.ecosystem.name(),
                    dependency.name,
                    datum.version
                )),
            }
        }
        self.0.blind.extend(blind);
    }
}

pub fn lagging(dependencies: &Dependencies) -> usize {
    dependencies
        .held
        .iter()
        .filter(|dependency| behind(dependency))
        .count()
}

fn behind(dependency: &Dependency) -> bool {
    let judged = plumb_cli::judge(dependency);
    let [Verdict::Stale { resolution, latest }] = judged.as_slice() else {
        return false;
    };
    match (Version::parse(resolution), Version::parse(latest)) {
        (Ok(resolution), Ok(latest)) => resolution < latest,
        _ => false,
    }
}

pub(super) fn retired(name: &str) -> bool {
    set::current().retired.iter().any(|(held, _)| held == name)
}

#[cfg(test)]
mod tests {
    use super::{Dependencies, Dependency};

    fn entry(seat: &str, resolution: &str, latest: Option<&str>) -> Dependency {
        Dependency {
            ecosystem: plumb_cli::Ecosystem::Cargo,
            name: "plumb".into(),
            requirement: "^0".into(),
            pinned: false,
            resolution: resolution.into(),
            latest: latest.map(str::to_string),
            seat: seat.into(),
        }
    }

    #[test]
    fn lagging() {
        let dependencies = Dependencies {
            held: vec![
                entry("a/Cargo.toml", "0.129.0", Some("0.130.0")),
                entry("b/Cargo.toml", "0.129.0", Some("0.130.0")),
                entry("c/Cargo.toml", "0.130.0", Some("0.130.0")),
                entry("d/Cargo.toml", "0.131.0", Some("0.130.0")),
                entry("e/Cargo.toml", "0.129.0", None),
                entry("f/Cargo.toml", "not-a-version", Some("0.130.0")),
            ],
            ..Default::default()
        };
        assert_eq!(super::lagging(&dependencies), 2);
    }
}
