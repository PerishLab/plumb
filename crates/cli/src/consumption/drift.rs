use plumb::depot::Rules;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SEAT: &str = "crates/cli/rules";

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Report {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl Report {
    pub fn empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    pub fn summary(&self, mark: &str) -> String {
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!("added: {}", self.added.join(", ")));
        }
        if !self.changed.is_empty() {
            parts.push(format!("changed: {}", self.changed.join(", ")));
        }
        if !self.removed.is_empty() {
            parts.push(format!("removed: {}", self.removed.join(", ")));
        }
        let short = &mark[..mark.len().min(12)];
        format!(
            "local {SEAT} differs from standing Depot configuration {short} ({}); consign configuration before releasing plumb",
            parts.join("; ")
        )
    }
}

pub fn check(root: &Path) -> Result<Option<(Report, String)>, String> {
    let rules = plumb::depot::rules()?;
    if rules.pinned().is_none() {
        return Ok(None);
    }
    let Some(report) = compare(root, rules)? else {
        return Ok(None);
    };
    if report.empty() {
        Ok(None)
    } else {
        Ok(Some((report, rules.mark().to_string())))
    }
}

pub fn compare(root: &Path, rules: &Rules) -> Result<Option<Report>, String> {
    let dir = root.join(SEAT);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut local = BTreeMap::new();
    walk(&dir, &dir, &mut local)?;
    let standing = rules
        .files()
        .iter()
        .filter(|(path, _)| path.starts_with("rules/"))
        .map(|(path, bytes)| (path.clone(), bytes.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut report = Report::default();
    for (path, bytes) in &local {
        match standing.get(path) {
            None => report.added.push(path.clone()),
            Some(held) if *held != bytes.as_slice() => report.changed.push(path.clone()),
            Some(_) => {}
        }
    }
    for path in standing.keys() {
        if !local.contains_key(path) {
            report.removed.push(path.clone());
        }
    }
    Ok(Some(report))
}

fn walk(root: &Path, dir: &Path, held: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|error| format!("cannot read {}: {error}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if meta.file_type().is_dir() {
            walk(root, &path, held)?;
            continue;
        }
        if !meta.file_type().is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| format!("{} escapes {}", path.display(), root.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes =
            fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        held.insert(format!("rules/{relative}"), bytes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Report, compare};
    use plumb::depot::{Rules, v3};
    use std::collections::BTreeMap;
    use std::fs;

    fn rules(entries: &[(&str, &str)]) -> Rules {
        let mut bodies = BTreeMap::new();
        let mut objects = Vec::new();
        for (path, body) in entries {
            let bytes = body.as_bytes().to_vec();
            objects.push(v3::Object {
                path: (*path).to_string(),
                sha256: plumb::depot::sha(&bytes),
                size: bytes.len() as u64,
                media: "application/toml".to_string(),
                executable: false,
            });
            bodies.insert((*path).to_string(), bytes);
        }
        let identity = v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: "v0.169.0".into(),
            marker: v3::Marker {
                name: "v0.169.0".into(),
                sha256: "0".repeat(64),
            },
            kind: v3::Kind::Configuration,
        };
        let manifest = v3::Manifest::new(identity, objects).expect("manifest");
        Rules::bundle(&v3::Bundle { manifest, bodies }).expect("rules")
    }

    #[test]
    fn synced() {
        let fixture = tempfile::tempdir().expect("fixture");
        let dir = fixture.path().join("crates/cli/rules/atoms");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(
            fixture.path().join("crates/cli/rules/catalog.toml"),
            "a = 1\n",
        )
        .expect("cat");
        fs::write(dir.join("base.toml"), "b = 2\n").expect("atom");
        let standing = rules(&[
            ("rules/atoms/base.toml", "b = 2\n"),
            ("rules/catalog.toml", "a = 1\n"),
        ]);
        let report = compare(fixture.path(), &standing)
            .expect("compare")
            .expect("present");
        assert!(report.empty());
    }

    #[test]
    fn drifted() {
        let fixture = tempfile::tempdir().expect("fixture");
        let dir = fixture.path().join("crates/cli/rules/atoms");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(
            fixture.path().join("crates/cli/rules/catalog.toml"),
            "a = 2\n",
        )
        .expect("cat");
        fs::write(dir.join("extra.toml"), "c = 3\n").expect("extra");
        let standing = rules(&[
            ("rules/atoms/old.toml", "b = 1\n"),
            ("rules/catalog.toml", "a = 1\n"),
        ]);
        let report = compare(fixture.path(), &standing)
            .expect("compare")
            .expect("present");
        assert_eq!(
            report,
            Report {
                added: vec!["rules/atoms/extra.toml".into()],
                removed: vec!["rules/atoms/old.toml".into()],
                changed: vec!["rules/catalog.toml".into()],
            }
        );
        let summary = report.summary("0123456789abcdef0123");
        assert!(summary.contains("rules/atoms/extra.toml"), "{summary}");
        assert!(summary.contains("rules/catalog.toml"), "{summary}");
        assert!(summary.contains("rules/atoms/old.toml"), "{summary}");
    }

    #[test]
    fn absent() {
        let fixture = tempfile::tempdir().expect("fixture");
        let standing = rules(&[("rules/catalog.toml", "a = 1\n")]);
        assert!(
            compare(fixture.path(), &standing)
                .expect("absent")
                .is_none()
        );
    }

    #[test]
    fn channels() {
        use super::super::{STABLE, pick};
        use std::cell::RefCell;

        let chosen = pick("v0.170.0-rc.1", |name| match name {
            "rc" => Ok("v0.170.0-rc.1".into()),
            STABLE => Ok("v0.169.0".into()),
            other => Err(other.into()),
        })
        .expect("rc");
        assert_eq!(chosen, "rc");

        let beta = pick("v0.170.0-beta.2", |name| match name {
            "beta" => Ok("v0.170.0-beta.2".into()),
            STABLE => Ok("v0.169.0".into()),
            other => Err(other.into()),
        })
        .expect("beta");
        assert_eq!(beta, "beta");

        let absent = pick("v0.170.0-rc.1", |name| match name {
            "rc" => Err("absent".into()),
            STABLE => Ok("v0.169.0".into()),
            other => Err(other.into()),
        })
        .expect("absent");
        assert_eq!(absent, STABLE);

        for staged in ["v0.168.0-rc.1", "v0.169.0-rc.1", "not-semver"] {
            let older = pick("v0.170.0-rc.1", |name| match name {
                "rc" => Ok(staged.into()),
                STABLE => Ok("v0.169.0".into()),
                other => Err(other.into()),
            })
            .expect(staged);
            assert_eq!(older, STABLE);
        }

        let calls = RefCell::new(Vec::new());
        for binary in ["v0.170.0", "v0.0.0", "0.170.0"] {
            let stable = pick(binary, |name| {
                calls.borrow_mut().push(name.to_string());
                Ok("v0.999.0-rc.1".into())
            })
            .expect(binary);
            assert_eq!(stable, STABLE);
        }
        assert!(calls.borrow().is_empty());

        let source = "https://depot.plumb.perish.uk";
        let manifest = v3::Manifest::new(
            v3::Identity {
                product: "plumb".into(),
                channel: "rc".into(),
                version: "v0.170.0-rc.1".into(),
                marker: v3::Marker {
                    name: "v0.170.0-rc.1".into(),
                    sha256: "a".repeat(64),
                },
                kind: v3::Kind::Configuration,
            },
            vec![v3::Object {
                path: "rules/catalog.toml".into(),
                sha256: plumb::depot::sha(b"a = 1\n"),
                size: 6,
                media: "application/toml".into(),
                executable: false,
            }],
        )
        .expect("manifest");
        let pointer = v3::Pointer::new(
            &manifest,
            v3::Publication {
                source,
                prior: None,
                created: "2026-11-27T00:00:00Z".into(),
            },
        )
        .expect("pointer");
        let pinned = v3::Pinned::pointer(&pointer).expect("rc pinned");
        let encoded = pinned.encode().expect("encode");
        let parsed = v3::Pinned::parse(&encoded, source).expect("explicit pin");
        assert_eq!(parsed.channel, "rc");
        assert_eq!(parsed.version, "v0.170.0-rc.1");
    }
}
