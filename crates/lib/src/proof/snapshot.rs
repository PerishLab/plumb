use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Output;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    mode: String,
    oid: String,
    path: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    root: PathBuf,
    entries: Vec<Entry>,
    untracked: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Refusal {
    pub kind: &'static str,
    pub message: String,
}

impl Display for Refusal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

impl Snapshot {
    pub fn read(root: &Path) -> Result<Self, Refusal> {
        let mut held = Self::listed(root)?;
        for entry in &mut held.entries {
            entry.bytes = bytes(root, entry)?;
        }
        Ok(held)
    }

    pub fn staged(root: &Path) -> Result<Self, Refusal> {
        let mut held = Self::listed(root)?;
        for entry in &mut held.entries {
            entry.bytes = if entry.mode == "160000" {
                entry.oid.as_bytes().to_vec()
            } else {
                listing(
                    root,
                    &["cat-file", "blob", &entry.oid],
                    "cannot read staged object",
                )?
            };
        }
        Ok(held)
    }

    fn listed(root: &Path) -> Result<Self, Refusal> {
        let listed = listing(
            root,
            &["ls-files", "--stage", "-z"],
            "cannot list tracked paths",
        )?;
        let entries = listed
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
            .map(Entry::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            root: root.to_path_buf(),
            entries,
            untracked: paths(root)?,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn untracked(&self) -> &[String] {
        &self.untracked
    }

    pub fn seat(&self, path: &str) -> Vec<&Entry> {
        let prefix = format!("{}/", path.trim_end_matches('/'));
        self.entries
            .iter()
            .filter(|entry| entry.path == path || entry.path.starts_with(&prefix))
            .collect()
    }
}

impl Entry {
    fn parse(record: &[u8]) -> Result<Self, Refusal> {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(refuse("git", "Git emitted a malformed index entry"));
        };
        let meta = std::str::from_utf8(&record[..tab])
            .map_err(|_| refuse("git", "Git emitted non-UTF-8 index metadata"))?;
        let mut fields = meta.split(' ');
        let mode = fields.next().unwrap_or_default();
        let oid = fields.next().unwrap_or_default();
        let stage = fields.next().unwrap_or_default();
        if mode.is_empty() || oid.is_empty() {
            return Err(refuse(
                "git",
                "Git index contains a malformed or unresolved entry",
            ));
        }
        if stage != "0" || fields.next().is_some() {
            return Err(refuse(
                "git",
                "Git index contains a malformed or unresolved entry",
            ));
        }
        let path = std::str::from_utf8(&record[tab + 1..])
            .map_err(|_| refuse("git-path", "tracked path is not UTF-8"))?;
        Ok(Self {
            mode: mode.to_owned(),
            oid: oid.to_owned(),
            path: path.to_owned(),
            bytes: Vec::new(),
        })
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn oid(&self) -> &str {
        &self.oid
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn listing(root: &Path, args: &[&str], fallback: &str) -> Result<Vec<u8>, Refusal> {
    let output = crate::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| refuse("git", format!("cannot execute git: {error}")))?;
    success(output, fallback)
}

fn paths(root: &Path) -> Result<Vec<String>, Refusal> {
    let listed = listing(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        "cannot list untracked paths",
    )?;
    listed
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            std::str::from_utf8(record)
                .map(str::to_owned)
                .map_err(|_| refuse("git-path", "untracked path is not UTF-8"))
        })
        .collect()
}

fn bytes(root: &Path, entry: &Entry) -> Result<Vec<u8>, Refusal> {
    if entry.mode == "160000" {
        return Ok(entry.oid.as_bytes().to_vec());
    }
    let path = root.join(&entry.path);
    if entry.mode == "120000" {
        let target = std::fs::read_link(&path).map_err(|error| unread(&entry.path, error))?;
        return native(target.as_os_str()).ok_or_else(|| {
            refuse(
                "tracked",
                format!("symlink {} has a non-UTF-8 target", entry.path),
            )
        });
    }
    std::fs::read(&path).map_err(|error| unread(&entry.path, error))
}

#[cfg(unix)]
fn native(path: &std::ffi::OsStr) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;

    Some(path.as_bytes().to_vec())
}

#[cfg(not(unix))]
fn native(path: &std::ffi::OsStr) -> Option<Vec<u8>> {
    path.to_str().map(|value| value.as_bytes().to_vec())
}

fn success(output: Output, fallback: &str) -> Result<Vec<u8>, Refusal> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(refuse(
        "git",
        if stderr.is_empty() {
            fallback.to_owned()
        } else {
            stderr
        },
    ))
}

fn unread(path: &str, error: std::io::Error) -> Refusal {
    refuse(
        "tracked",
        format!("cannot read tracked path {path}: {error}"),
    )
}

pub(crate) fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        message: message.into(),
    }
}
