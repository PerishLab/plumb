use super::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub use super::snapshot::Refusal;

pub const CODEC: &str = "p64-v1";
pub const SCHEMA: &str = "plumb.vocabulary/v2";
const RULE: &str = "rules/atoms/vocabulary.toml";

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
    pub digest: String,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    retired: Vec<String>,
}

struct Closure<'a> {
    snapshot: &'a Snapshot,
    dictionary: &'a Dictionary,
}

impl Dictionary {
    pub fn synced() -> Result<Self, Refusal> {
        let seat = crate::depot::rules().map_err(|error| refuse("depot", error))?;
        let source = seat.read(RULE).map_err(|error| refuse("depot", error))?;
        Self::parse(&source)
    }

    pub fn parse(source: &str) -> Result<Self, Refusal> {
        let raw = toml::from_str::<Raw>(source).map_err(|error| {
            refuse(
                "dictionary",
                format!("cannot parse domain dictionary: {error}"),
            )
        })?;
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
    let dictionary = Dictionary::synced()?;
    scan(root, &dictionary)
}

pub fn scan(root: &Path, dictionary: &Dictionary) -> Result<Report, Refusal> {
    if dictionary.terms.is_empty() {
        return Ok(report(dictionary, Coverage::default(), Vec::new()));
    }
    let snapshot = Snapshot::read(root)?;
    sift(&snapshot, dictionary)
}

pub fn observe(snapshot: &Snapshot) -> Result<Report, Refusal> {
    let dictionary = Dictionary::synced()?;
    sift(snapshot, &dictionary)
}

pub fn sift(snapshot: &Snapshot, dictionary: &Dictionary) -> Result<Report, Refusal> {
    Closure {
        snapshot,
        dictionary,
    }
    .scan()
}

fn report(dictionary: &Dictionary, coverage: Coverage, hits: Vec<Hit>) -> Report {
    Report {
        schema: SCHEMA,
        codec: CODEC,
        digest: dictionary.digest.clone(),
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
        let entries = self.snapshot.entries();
        let mut coverage = Coverage {
            tracked: entries.len(),
            ..Coverage::default()
        };
        let mut hits = Vec::new();
        for entry in entries {
            hits.extend(matches(
                entry.path(),
                entry.path().as_bytes(),
                "path",
                self.dictionary,
            ));
            if entry.mode() != "160000" {
                hits.extend(matches(
                    entry.path(),
                    entry.bytes(),
                    "content",
                    self.dictionary,
                ));
            }
            coverage.scanned += 1;
        }
        Ok(report(self.dictionary, coverage, hits))
    }
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

pub(super) fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    super::snapshot::refuse(kind, message)
}
