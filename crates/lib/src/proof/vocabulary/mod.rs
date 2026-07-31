use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::process::{Command, Output};

pub const CODEC: &str = "p64-v1";
pub const SCHEMA: &str = "plumb.vocabulary/v1";
const BUNDLED: &str = include_str!("../../../rules/vocabulary.toml");

mod codec;

pub use codec::{decode, encode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dictionary {
    digest: String,
    terms: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub codec: &'static str,
    pub dictionary_digest: String,
    pub retired: usize,
    pub coverage: Coverage,
    pub hits: Vec<Hit>,
    pub ok: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Coverage {
    pub tracked: usize,
    pub scanned: usize,
    pub exempt: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Hit {
    pub path: String,
    pub surface: &'static str,
    pub term: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Refusal {
    pub kind: &'static str,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: u8,
    codec: String,
    retired: Vec<String>,
}

struct Entry {
    mode: String,
    path: String,
}

struct Closure<'a> {
    root: &'a Path,
    dictionary: &'a Dictionary,
}

impl Display for Refusal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

impl Dictionary {
    pub fn bundled() -> Result<Self, Refusal> {
        Self::parse(BUNDLED)
    }

    pub fn parse(source: &str) -> Result<Self, Refusal> {
        let raw = toml::from_str::<Raw>(source).map_err(|error| {
            refuse(
                "dictionary",
                format!("cannot parse domain dictionary: {error}"),
            )
        })?;
        if raw.schema != 1 || raw.codec != CODEC {
            return Err(refuse(
                "dictionary",
                "domain dictionary schema or codec is unsupported",
            ));
        }
        let mut unique = BTreeSet::new();
        let mut terms = Vec::new();
        for encoded in raw.retired {
            let term = decode(&encoded)?;
            if !unique.insert(term.clone()) {
                return Err(refuse(
                    "dictionary",
                    "domain dictionary decodes one term more than once",
                ));
            }
            terms.push(term);
        }
        terms.sort();
        Ok(Self {
            digest: codec::digest(source.as_bytes()),
            terms,
        })
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn terms(&self) -> &[String] {
        &self.terms
    }
}

pub fn inspect(root: &Path) -> Result<Report, Refusal> {
    let dictionary = Dictionary::bundled()?;
    scan(root, &dictionary)
}

pub fn scan(root: &Path, dictionary: &Dictionary) -> Result<Report, Refusal> {
    Closure { root, dictionary }.scan()
}

fn report(dictionary: &Dictionary, coverage: Coverage, hits: Vec<Hit>) -> Report {
    Report {
        schema: SCHEMA,
        codec: CODEC,
        dictionary_digest: dictionary.digest.clone(),
        retired: dictionary.terms.len(),
        ok: hits.is_empty(),
        coverage,
        hits,
    }
}

impl Closure<'_> {
    fn scan(&self) -> Result<Report, Refusal> {
        if self.dictionary.terms.is_empty() {
            return Ok(report(self.dictionary, Coverage::default(), Vec::new()));
        }
        let entries = self.tracked()?;
        let mut coverage = Coverage {
            tracked: entries.len(),
            ..Coverage::default()
        };
        let mut hits = Vec::new();
        for entry in entries {
            if exempt(&entry.path) {
                coverage.exempt += 1;
                continue;
            }
            hits.extend(matches(
                &entry.path,
                entry.path.as_bytes(),
                "path",
                self.dictionary,
            ));
            if entry.mode != "160000" {
                let bytes = self.bytes(&entry)?;
                hits.extend(matches(&entry.path, &bytes, "content", self.dictionary));
            }
            coverage.scanned += 1;
        }
        Ok(report(self.dictionary, coverage, hits))
    }

    fn tracked(&self) -> Result<Vec<Entry>, Refusal> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["ls-files", "--stage", "-z"])
            .output()
            .map_err(|error| refuse("git", format!("cannot execute git: {error}")))?;
        let bytes = success(output, "cannot list tracked paths")?;
        bytes
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
            .map(Entry::parse)
            .collect()
    }

    fn bytes(&self, entry: &Entry) -> Result<Vec<u8>, Refusal> {
        let path = self.root.join(&entry.path);
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
        let _oid = fields.next().unwrap_or_default();
        let stage = fields.next().unwrap_or_default();
        if mode.is_empty() || stage != "0" || fields.next().is_some() {
            return Err(refuse(
                "git",
                "Git index contains a malformed or unresolved entry",
            ));
        }
        let path = std::str::from_utf8(&record[tab + 1..])
            .map_err(|_| refuse("git-path", "tracked path is not UTF-8"))?;
        Ok(Self {
            mode: mode.to_owned(),
            path: path.to_owned(),
        })
    }
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

fn matches(path: &str, bytes: &[u8], surface: &'static str, dictionary: &Dictionary) -> Vec<Hit> {
    dictionary
        .terms
        .iter()
        .filter(|term| contains(bytes, term.as_bytes()))
        .map(|term| Hit {
            path: path.to_owned(),
            surface,
            term: term.clone(),
        })
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

fn exempt(path: &str) -> bool {
    path == "docs/CHANGELOG" || path.starts_with("docs/CHANGELOG/")
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

pub(super) fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        message: message.into(),
    }
}
