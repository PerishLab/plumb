use super::record::{Dependency, Repository, Skip};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const WATCHED: [&str; 13] = [
    "radicula-lock",
    "fs2",
    "fs4",
    "fd-lock",
    "file-guard",
    "tokio",
    "parking_lot",
    "async-lock",
    "spin",
    "once_cell",
    "libc",
    "nix",
    "rustix",
];

const LANGUAGES: [(&str, &str); 30] = [
    ("py", "python"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("mts", "typescript"),
    ("cts", "typescript"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("svelte", "svelte"),
    ("go", "go"),
    ("sh", "shell"),
    ("bash", "shell"),
    ("c", "c"),
    ("h", "c"),
    ("cc", "cpp"),
    ("cpp", "cpp"),
    ("java", "java"),
    ("swift", "swift"),
    ("ps1", "powershell"),
    ("m", "objective-c"),
    ("metal", "metal"),
    ("uc", "ucode"),
    ("sql", "sql"),
    ("html", "html"),
    ("tpl", "template"),
    ("pt", "template"),
    ("in", "template"),
    ("g4", "grammar"),
    ("grammar", "grammar"),
];

pub struct Source {
    pub files: Vec<String>,
    pub repository: Repository,
}

pub struct Package {
    pub record: Dependency,
    pub directory: Option<PathBuf>,
}

pub struct Checkout<'a> {
    pub root: &'a Path,
}

impl Checkout<'_> {
    pub fn read(&self) -> Source {
        let root = self.root;
        let listed = self.git(&[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ]);
        let mut files: Vec<String> = match listed {
            Some(text) => text
                .split('\0')
                .filter(|path| !path.is_empty() && root.join(path).is_file())
                .map(str::to_string)
                .collect(),
            None => self.walk(Path::new("")),
        };
        files.sort();
        files.dedup();
        let commit = self
            .git(&["rev-parse", "--verify", "HEAD"])
            .map(|text| text.trim().to_string());
        let dirty = self
            .git(&["status", "--porcelain"])
            .is_some_and(|text| !text.trim().is_empty());
        let name = self
            .git(&["remote", "get-url", "origin"])
            .map(|url| named(url.trim()))
            .unwrap_or_else(|| {
                root.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
        Source {
            files,
            repository: Repository {
                name,
                commit,
                dirty,
            },
        }
    }

    pub fn packages(&self, files: &[String]) -> Vec<Package> {
        let mut seen = BTreeSet::new();
        let mut found = Vec::new();
        for path in files.iter().filter(|path| path.ends_with("Cargo.lock")) {
            let Some(table) = std::fs::read_to_string(self.root.join(path))
                .ok()
                .and_then(|text| text.parse::<toml::Table>().ok())
            else {
                continue;
            };
            let listed = table.get("package").and_then(toml::Value::as_array);
            let fresh = listed.into_iter().flatten().filter_map(package);
            found.extend(fresh.filter(|package| seen.insert(package.record.clone())));
        }
        found.sort_by(|left, right| left.record.cmp(&right.record));
        found
    }

    fn walk(&self, relative: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.root.join(relative)) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "target" {
                continue;
            }
            let path = relative.join(&name);
            if entry.path().is_dir() {
                found.extend(self.walk(&path));
            } else {
                found.push(path.to_string_lossy().replace('\\', "/"));
            }
        }
        found
    }

    fn git(&self, arguments: &[&str]) -> Option<String> {
        let output = plumb::config::detached("git")
            .arg("--no-optional-locks")
            .arg("-C")
            .arg(self.root)
            .args(arguments)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }
}

pub fn foreign(files: &[String]) -> Vec<Skip> {
    files
        .iter()
        .filter_map(|path| {
            let extension = Path::new(path).extension()?.to_str()?;
            let (_, language) = LANGUAGES.iter().find(|(known, _)| *known == extension)?;
            Some(Skip {
                path: path.clone(),
                reason: format!("{language} source is outside the Rust inspection"),
            })
        })
        .collect()
}

fn package(entry: &toml::Value) -> Option<Package> {
    let field = |key: &str| {
        entry
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    let name = field("name")?;
    let version = field("version")?;
    let source = field("source")?;
    let owned = source == crate::catalog::set::CARGO.index;
    if !owned && !WATCHED.contains(&name.as_str()) {
        return None;
    }
    let directory = owned.then(|| registry(&source, &name, &version)).flatten();
    Some(Package {
        record: Dependency {
            scanned: directory.is_some(),
            name,
            version,
            source,
        },
        directory,
    })
}

fn registry(source: &str, name: &str, version: &str) -> Option<PathBuf> {
    let host = source
        .trim_start_matches("sparse+")
        .trim_start_matches("https://")
        .trim_end_matches('/');
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".cargo")))?;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(home.join("registry").join("src"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .is_some_and(|label| label.to_string_lossy().starts_with(&format!("{host}-")))
        })
        .collect();
    entries.sort();
    entries
        .into_iter()
        .map(|path| path.join(format!("{name}-{version}")))
        .find(|path| path.join("Cargo.toml").is_file())
}

fn named(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = trimmed.rsplit(['/', ':']).take(2).collect();
    match parts.as_slice() {
        [name, owner] => format!("{owner}/{name}"),
        _ => trimmed.to_string(),
    }
}
