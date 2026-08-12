use std::io::ErrorKind;
use std::path::Path;

const CODE: [&str; 5] = ["css", "rs", "scss", "ts", "tsx"];

#[derive(Default)]
pub struct Metric {
    pub lines: usize,
    pub unread: Vec<String>,
}

struct Source<'a>(&'a Path);

pub fn read(root: &Path) -> Metric {
    Source(root).read()
}

impl Source<'_> {
    fn read(&self) -> Metric {
        let mut found = Metric::default();
        self.seats(self.0, &mut found, &["src", "lib", "app/src"]);
        for group in ["apps", "crates", "packages"] {
            let entries = match std::fs::read_dir(self.0.join(group)) {
                Ok(entries) => entries,
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(error) => {
                    found.unread.push(error.to_string());
                    continue;
                }
            };
            for entry in entries {
                match entry {
                    Ok(entry) => self.seats(&entry.path(), &mut found, &["src", "lib"]),
                    Err(error) => found.unread.push(error.to_string()),
                }
            }
        }
        found
    }

    fn seats(&self, root: &Path, found: &mut Metric, names: &[&str]) {
        for name in names {
            let path = root.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_dir() => self.lines(&path, found),
                Ok(meta) if meta.file_type().is_symlink() => found
                    .unread
                    .push(format!("source seat {} is a symlink", path.display())),
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => found.unread.push(error.to_string()),
            }
        }
    }

    fn lines(&self, path: &Path, found: &mut Metric) {
        let kind = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta.file_type(),
            Err(error) => {
                found.unread.push(error.to_string());
                return;
            }
        };
        if kind.is_symlink() {
            found
                .unread
                .push(format!("source path {} is a symlink", path.display()));
            return;
        }
        if kind.is_file() {
            let held = path.extension().and_then(|value| value.to_str());
            if !held.is_some_and(|suffix| CODE.contains(&suffix)) {
                return;
            }
            match std::fs::read(path) {
                Ok(bytes) => found.lines += count(&bytes),
                Err(error) => found.unread.push(error.to_string()),
            }
            return;
        }
        if !kind.is_dir() {
            let held = path.extension().and_then(|value| value.to_str());
            if held.is_some_and(|suffix| CODE.contains(&suffix)) {
                found.unread.push(format!(
                    "source path {} is not a regular file",
                    path.display()
                ));
            }
            return;
        }
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) => {
                found.unread.push(error.to_string());
                return;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry) => self.lines(&entry.path(), found),
                Err(error) => found.unread.push(error.to_string()),
            }
        }
    }
}

fn count(bytes: &[u8]) -> usize {
    let lines = bytes.iter().filter(|byte| **byte == b'\n').count();
    lines + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}
