use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

const TONGUES: [&str; 2] = ["en", "zh"];
const LEAVES: [&str; 2] = ["INDEX.md", "MIGRATION.md"];

mod text;

use text::{count, digest, flat};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Proof {
    pub base: String,
    pub candidate: String,
    pub diff: String,
    pub units: usize,
    pub languages: BTreeMap<String, Language>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Language {
    pub lines: usize,
    pub budget: usize,
    pub index: String,
    pub migration: String,
}

pub struct Claim<'a> {
    pub root: &'a Path,
    pub home: &'a Path,
    pub version: &'a str,
    pub previous: Option<&'a str>,
    pub candidate: &'a str,
}

struct Repo<'a>(&'a Path);

pub fn prove(input: Claim<'_>) -> Result<Proof, String> {
    if !valid(input.candidate) {
        return Err(format!(
            "invalid changelog candidate commit: {}",
            input.candidate
        ));
    }
    let repo = Repo(input.root);
    let base = repo.base(input.previous, input.candidate)?;
    let mut units = repo.names(&base, input.candidate)?.len();
    let mut identity = Vec::new();
    for row in repo.stat(&base, input.candidate)? {
        units += row.added.parse::<usize>().unwrap_or(0);
        units += row.removed.parse::<usize>().unwrap_or(0);
        push(&mut identity, row.path.as_bytes());
        push(&mut identity, row.added.as_bytes());
        push(&mut identity, row.removed.as_bytes());
    }
    let budget = (4 * scale(units)).clamp(120, 800);
    let mut languages = BTreeMap::new();
    for tongue in TONGUES {
        languages.insert(
            tongue.to_string(),
            language(input.home, tongue, budget, units)?,
        );
    }
    Ok(Proof {
        base,
        candidate: input.candidate.to_string(),
        diff: digest(&identity),
        units,
        languages,
    })
}

impl Repo<'_> {
    fn base(&self, previous: Option<&str>, candidate: &str) -> Result<String, String> {
        let Some(base) = previous else {
            return self.empty();
        };
        if !valid(base) {
            return Err(format!("invalid changelog base commit: {base}"));
        }
        let ancestor = self.git(["merge-base", "--is-ancestor", base, candidate])?;
        if !ancestor.status.success() {
            return Err(format!(
                "previous stable {base} is not an ancestor of candidate {candidate}"
            ));
        }
        Ok(base.to_string())
    }

    fn names(&self, base: &str, candidate: &str) -> Result<Vec<String>, String> {
        let bytes = success(
            self.git(["diff", "--no-renames", "--name-only", "-z", base, candidate])?,
            "cannot list changelog release diff",
        )?;
        bytes
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| {
                std::str::from_utf8(path)
                    .map(str::to_string)
                    .map_err(|_| "release diff contains a non-UTF-8 path".to_string())
            })
            .collect()
    }

    fn stat(&self, base: &str, candidate: &str) -> Result<Vec<Row>, String> {
        let bytes = success(
            self.git(["diff", "--no-renames", "--numstat", "-z", base, candidate])?,
            "cannot measure changelog release diff",
        )?;
        bytes
            .split(|byte| *byte == 0)
            .filter(|row| !row.is_empty())
            .map(Row::parse)
            .collect()
    }

    fn empty(&self) -> Result<String, String> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(["hash-object", "-t", "tree", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        drop(child.stdin.take());
        success(
            child
                .wait_with_output()
                .map_err(|error| format!("cannot wait for git: {error}"))?,
            "cannot derive the empty tree",
        )
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map(|text| text.trim().to_string())
                .map_err(|_| "empty tree identity is not UTF-8".to_string())
        })
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<Output, String> {
        Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))
    }
}

struct Row {
    added: String,
    removed: String,
    path: String,
}

impl Row {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| "release numstat contains non-UTF-8 evidence".to_string())?;
        let mut fields = text.splitn(3, '\t');
        let added = fields.next().unwrap_or_default().to_string();
        let removed = fields.next().unwrap_or_default().to_string();
        let path = fields.next().unwrap_or_default().to_string();
        if path.is_empty() {
            return Err("release numstat is malformed".into());
        }
        Ok(Self {
            added,
            removed,
            path,
        })
    }
}

fn language(home: &Path, tongue: &str, budget: usize, units: usize) -> Result<Language, String> {
    let mut lines = 0;
    let mut seals = BTreeMap::new();
    for leaf in LEAVES {
        let path = home.join(tongue).join(leaf);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| format!("{} is not UTF-8", path.display()))?;
        if text.trim().is_empty() {
            return Err(format!("{} is empty", path.display()));
        }
        lines += count(text.as_bytes());
        seals.insert(leaf, digest(&flat(&bytes)));
    }
    if lines > budget {
        return Err(format!(
            "changelog {} {tongue} has {lines} lines, above diff budget {budget} for {units} units",
            home.display()
        ));
    }
    Ok(Language {
        lines,
        budget,
        index: seals.remove("INDEX.md").unwrap_or_default(),
        migration: seals.remove("MIGRATION.md").unwrap_or_default(),
    })
}

fn success(output: Output, fallback: &str) -> Result<Vec<u8>, String> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if error.is_empty() {
        fallback.to_string()
    } else {
        error
    })
}

fn valid(value: &str) -> bool {
    if !(40..=64).contains(&value.len()) {
        return false;
    }
    value
        .chars()
        .all(|held| held.is_ascii_digit() || ('a'..='f').contains(&held))
}

fn scale(value: usize) -> usize {
    (value as f64).sqrt().ceil() as usize
}

fn push(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(value.len().to_string().as_bytes());
    target.push(0);
    target.extend_from_slice(value);
}
