use super::bucket::Record;
use super::graph;
use super::sweep::IDLE;
use std::path::{Path, PathBuf};

pub(super) struct Target {
    pub key: Option<String>,
    pub paths: Vec<PathBuf>,
    pub bytes: u64,
}

impl Target {
    fn new(key: Option<String>, paths: Vec<PathBuf>) -> Self {
        let bytes = paths.iter().map(|path| graph::Node(path).size()).sum();
        Self { key, paths, bytes }
    }
}

#[derive(Default)]
pub(super) struct Plan {
    pub targets: Vec<Target>,
    pub working: u64,
    pub superseded: u64,
}

pub(super) struct Seat<'a>(pub &'a Path);

impl Seat<'_> {
    pub fn whole(&self) -> Result<Plan, String> {
        Ok(Plan {
            targets: graph::Node(self.0)
                .listing()?
                .into_iter()
                .filter(|path| !matches!(graph::Node(path).name().as_str(), "lease" | "plumb.json"))
                .map(|path| Target::new(None, vec![path]))
                .collect(),
            ..Plan::default()
        })
    }

    pub fn live(&self, record: &Record, now: u64, span: u64) -> Result<Plan, String> {
        let mut plan = Plan::default();
        let mut held = Vec::new();
        for profile in graph::profiles(self.0)? {
            for key in profile.units.keys() {
                held.push(Target::new(
                    Some(format!("{}/{key}", profile.name)),
                    profile.files(key),
                ));
            }
            for (key, path, _) in profile.incremental()? {
                held.push(Target::new(
                    Some(format!("{}/{key}", profile.name)),
                    vec![path],
                ));
            }
            plan.targets.extend(
                profile
                    .orphans()?
                    .into_iter()
                    .map(|path| Target::new(None, vec![path])),
            );
        }
        let used = |target: &Target| {
            target
                .key
                .as_ref()
                .and_then(|key| record.units.get(key).copied())
        };
        let latest = held.iter().filter_map(used).max().unwrap_or(0);
        let mut superseded = Vec::new();
        for target in held {
            match used(&target) {
                Some(stamp) if now.saturating_sub(stamp) > IDLE => plan.targets.push(target),
                Some(stamp) if stamp == latest => plan.working += target.bytes,
                Some(stamp) => superseded.push((stamp, target)),
                None => {}
            }
        }
        superseded.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.key.cmp(&right.1.key)));
        let mut kept = plan.working;
        for (_, target) in superseded {
            plan.superseded += target.bytes;
            kept += target.bytes;
            if plan.working > 0 && kept > span * plan.working {
                plan.targets.push(target);
            }
        }
        Ok(plan)
    }

    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(self.0)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

pub(super) fn execute(targets: &[Target], record: &mut Record) -> (u64, Option<String>) {
    let mut freed = 0;
    for target in targets {
        for path in &target.paths {
            let bytes = graph::Node(path).size();
            if let Err(error) = remove(path) {
                return (
                    freed,
                    Some(format!("cannot remove {}: {error}", path.display())),
                );
            }
            freed += bytes;
        }
        if let Some(key) = &target.key {
            record.units.remove(key);
        }
    }
    (freed, None)
}

fn remove(path: &Path) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}
