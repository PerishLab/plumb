use plumb::depot::v3::{Channel, Kind};
use std::cmp::Ordering;
use std::fs;
use std::path::{Component, Path};

pub(super) struct Check<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub marker: &'a str,
    pub generation: &'a str,
}

struct Tree<'a>(&'a Path);

pub(super) fn vet(dir: &Path) -> Result<(), String> {
    Tree(dir).vet()
}

impl Tree<'_> {
    fn vet(&self) -> Result<(), String> {
        let root = fs::symlink_metadata(self.0)
            .map_err(|error| format!("cannot inspect {}: {error}", self.0.display()))?;
        if root.file_type().is_symlink() || !root.file_type().is_dir() {
            return Err(format!(
                "configuration {} is not a plain directory",
                self.0.display()
            ));
        }
        let mut files = 0usize;
        self.walk(self.0, &mut files)?;
        if files == 0 {
            return Err(format!("configuration {} holds no files", self.0.display()));
        }
        Ok(())
    }

    fn walk(&self, at: &Path, files: &mut usize) -> Result<(), String> {
        let mut entries = fs::read_dir(at)
            .map_err(|error| format!("cannot read {}: {error}", at.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot read {}: {error}", at.display()))?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let kind = fs::symlink_metadata(&path)
                .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
                .file_type();
            if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                return Err(format!(
                    "configuration carries a special object: {}",
                    path.display()
                ));
            }
            let name = self.relative(&path)?;
            if kind.is_dir() {
                self.walk(&path, files)?;
                continue;
            }
            if path
                .extension()
                .and_then(|held| held.to_str())
                .is_some_and(|held| held.eq_ignore_ascii_case("toml"))
            {
                parse(&path, &name)?;
            }
            *files += 1;
        }
        Ok(())
    }

    fn relative(&self, path: &Path) -> Result<String, String> {
        let stripped = path
            .strip_prefix(self.0)
            .map_err(|_| format!("{} escapes {}", path.display(), self.0.display()))?;
        let parts = stripped
            .components()
            .map(|part| {
                let text = part
                    .as_os_str()
                    .to_str()
                    .ok_or_else(|| format!("{} is not UTF-8", path.display()))?;
                if !matches!(part, Component::Normal(_)) || text.contains('\\') {
                    return Err(format!(
                        "configuration path {} is not anchored",
                        path.display()
                    ));
                }
                Ok(text)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let joined = parts.join("/");
        if joined.is_empty() || joined.starts_with('/') || joined.contains('\\') {
            return Err(format!("configuration path {joined} is not anchored"));
        }
        if joined
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(format!("configuration path {joined} is not anchored"));
        }
        Ok(joined)
    }
}

impl Check<'_> {
    pub(super) fn verify(&self) -> Result<(), String> {
        let discovered = Channel {
            source: self.source,
            product: self.product,
            channel: self.channel,
            kind: Kind::Configuration,
        }
        .resolve()?;
        match compare(&discovered.pointer.version, self.marker)? {
            Ordering::Equal if discovered.pointer.generation == self.generation => Ok(()),
            Ordering::Equal => Err(format!(
                "channel pointer {} disagrees with standing {}",
                discovered.pointer.generation, self.generation
            )),
            Ordering::Greater => Ok(()),
            Ordering::Less => Err(format!(
                "channel pointer stayed on older {} below {}",
                discovered.pointer.version, self.marker
            )),
        }
    }
}

fn parse(path: &Path, relative: &str) -> Result<(), String> {
    let body = fs::read(path).map_err(|error| format!("cannot read {relative}: {error}"))?;
    if body.iter().all(u8::is_ascii_whitespace) {
        return Err(format!("configuration {relative} is empty"));
    }
    let text = String::from_utf8(body)
        .map_err(|error| format!("configuration {relative} is not valid TOML: {error}"))?;
    if text.trim().is_empty() {
        return Err(format!("configuration {relative} is empty"));
    }
    let parsed: toml::Table = toml::from_str(&text)
        .map_err(|error| format!("configuration {relative} is not valid TOML: {error}"))?;
    if parsed.is_empty() {
        return Err(format!("configuration {relative} holds no TOML entries"));
    }
    Ok(())
}

fn compare(channel: &str, marker: &str) -> Result<Ordering, String> {
    let version = |raw: &str| {
        raw.strip_prefix('v')
            .ok_or_else(|| format!("invalid version {raw}"))
            .and_then(|held| {
                semver::Version::parse(held)
                    .map_err(|error| format!("invalid version {raw}: {error}"))
            })
    };
    Ok(version(channel)?.cmp(&version(marker)?))
}

#[cfg(test)]
mod tests {
    use super::super::{Consign, gathered};
    use super::vet;
    use crate::shape::release::Spec;
    use plumb::depot::v3::Kind;
    use std::path::Path;

    fn spec(root: &Path, source: &str) -> Spec {
        let text = format!(
            "[identity]\nns = \"plumb\"\n[release]\nauthority = \"https://releases.plumb.example\"\nbinaries = [\"plumb\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n[release.depot]\nsource = \"{source}\"\n"
        );
        Spec::decode(root, &text, "fixture").expect("spec")
    }

    #[test]
    fn tree() {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path();
        assert!(vet(root).expect_err("empty").contains("no files"));
        let nested = root.join("rules");
        std::fs::create_dir(&nested).expect("dir");
        std::fs::write(nested.join("catalog.toml"), "answer = 42\n").expect("toml");
        std::fs::write(nested.join("NOTE.TXT"), "companion\n").expect("note");
        std::fs::write(nested.join("UPPER.TOML"), "flag = true\n").expect("upper");
        vet(root).expect("valid");
        let options = Consign {
            version: "v1.2.3".into(),
            kind: "configuration".into(),
            dir: Some(root.into()),
            unwatched: false,
            dry: true,
        };
        let (objects, kind) = gathered(
            &options,
            root,
            &spec(root, "https://depot.plumb.example"),
            "v1.2.3",
        )
        .expect("gathered");
        assert_eq!((kind, objects.len()), (Kind::Configuration, 3));
    }

    #[test]
    fn refused() {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path();
        let file = root.join("rules.toml");
        std::fs::write(&file, "# comment only\n").expect("comment");
        assert!(vet(root).expect_err("comment").contains("no TOML"));
        std::fs::write(&file, "not = [valid").expect("invalid");
        assert!(vet(root).expect_err("invalid").contains("valid TOML"));
        std::fs::write(&file, "   \n\t ").expect("blank");
        assert!(vet(root).expect_err("blank").contains("empty"));
        std::fs::remove_file(&file).expect("clean");
        std::fs::write(root.join("BAD.TOML"), "# empty\n").expect("upper");
        assert!(vet(root).expect_err("upper").contains("no TOML"));
    }

    #[test]
    #[cfg(unix)]
    fn symlink() {
        use std::os::unix::fs::symlink;
        let fixture = tempfile::tempdir().expect("fixture");
        std::fs::write(fixture.path().join("rules.toml"), "answer = 1\n").expect("toml");
        symlink("rules.toml", fixture.path().join("alias.toml")).expect("link");
        assert!(
            vet(fixture.path())
                .expect_err("link")
                .contains("special object")
        );
    }
}
