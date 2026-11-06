use super::wire::Wire;
use crate::semantics::Source;
use crate::windtun::{
    Declaration,
    qualification::{Fault, Read},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) struct Reader<'a, R>(pub &'a R);

pub(super) struct Subject {
    pub source: Source,
    pub repository: Value,
    pub files: Value,
    pub declaration: Option<Declaration>,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct Owner {
    login: String,
    id: u64,
    #[serde(rename = "node_id")]
    node: String,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct Repository {
    id: u64,
    #[serde(rename = "node_id")]
    node: String,
    #[serde(rename = "full_name")]
    name: String,
    owner: Owner,
    visibility: String,
    archived: bool,
    disabled: bool,
}

#[derive(Deserialize)]
struct Commit {
    sha: String,
    tree: Identity,
}

#[derive(Deserialize)]
struct Identity {
    sha: String,
}

#[derive(Deserialize)]
struct Tree {
    sha: String,
    truncated: bool,
    tree: Vec<Entry>,
}

#[derive(Deserialize, Serialize)]
struct Entry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
}

#[derive(Deserialize)]
struct Blob {
    sha: String,
    size: usize,
    encoding: String,
    content: String,
}

impl<R: Read> Reader<'_, R> {
    fn repository(&self, name: &str) -> Result<Repository, Fault> {
        let held: Repository =
            Wire::document(self.0, &format!("https://api.github.com/repos/{name}"))?;
        Wire::require(
            held.name == name && held.visibility == "public",
            "source repository identity differs",
        )?;
        Wire::require(
            !held.archived && !held.disabled,
            "source repository is inactive",
        )?;
        Wire::require(
            held.id > 0 && !held.node.is_empty(),
            "source repository has no native identity",
        )?;
        Wire::require(held.owner.login == "PerishLab", "foreign source owner")?;
        Wire::require(
            held.owner.id == 330236468 && held.owner.node == "O_kgDOE68CNA",
            "foreign native owner",
        )?;
        Ok(held)
    }

    fn commit(&self, repository: &str, sha: &str) -> Result<Commit, Fault> {
        let held: Commit = Wire::document(
            self.0,
            &format!("https://api.github.com/repos/{repository}/git/commits/{sha}"),
        )?;
        Wire::require(held.sha == sha, "source commit differs")?;
        Wire::hex(&held.tree.sha)?;
        Ok(held)
    }

    pub fn inspect(&self, repository: &str, commit: &str) -> Result<Subject, Fault> {
        let product = repository
            .strip_prefix("PerishLab/")
            .ok_or_else(|| Fault::refused("source requires a PerishLab repository"))?;
        Wire::name(product)?;
        Wire::hex(commit)?;
        let identity = self.repository(repository)?;
        let head = self.commit(repository, commit)?;
        let tree: Tree = Wire::document(
            self.0,
            &format!(
                "https://api.github.com/repos/{repository}/git/trees/{}?recursive=1",
                head.tree.sha
            ),
        )?;
        Wire::require(
            tree.sha == head.tree.sha && !tree.truncated,
            "source tree differs or is truncated",
        )?;
        let files = tree.files()?;
        let manifest = files
            .iter()
            .find(|entry| entry.path == "plumb.toml")
            .ok_or_else(|| Fault::unknown("source has no product declaration"))?;
        let raw = self.blob(repository, manifest)?;
        let text = std::str::from_utf8(&raw)
            .map_err(|_| Fault::refused("product declaration is not UTF-8"))?;
        let document: toml::Table = text
            .parse()
            .map_err(|_| Fault::refused("invalid product declaration"))?;
        Wire::require(
            document
                .get("identity")
                .and_then(|v| v.get("ns"))
                .and_then(|v| v.as_str())
                == Some(product),
            "product declaration names another source",
        )?;
        let declaration = crate::windtun::parse(text).map_err(Fault::refused)?;
        Ok(Subject {
            source: Source {
                repository: repository.into(),
                commit: commit.into(),
                tree: head.tree.sha.clone(),
            },
            repository: serde_json::to_value(&identity)
                .map_err(|_| Fault::refused("cannot encode native repository"))?,
            files: json!({"schema":"plumb.release.source/v1", "repository":identity, "tree":head.tree.sha, "files":files}),
            declaration,
        })
    }

    fn blob(&self, repository: &str, entry: &Entry) -> Result<Vec<u8>, Fault> {
        Wire::require(
            entry
                .size
                .is_some_and(|size| size <= (Wire::LIMIT / 2) as u64),
            "product declaration exceeds bounds",
        )?;
        let held: Blob = Wire::document(
            self.0,
            &format!(
                "https://api.github.com/repos/{repository}/git/blobs/{}",
                entry.sha
            ),
        )?;
        Wire::require(
            held.sha == entry.sha && held.encoding == "base64",
            "product declaration blob differs",
        )?;
        let encoded = held.content.lines().collect::<String>();
        let raw = STANDARD
            .decode(encoded)
            .map_err(|_| Fault::refused("invalid Git blob encoding"))?;
        Wire::require(
            raw.len() == held.size && Some(raw.len() as u64) == entry.size,
            "Git blob size differs",
        )?;
        Wire::require(Wire::blob(&raw) == entry.sha, "Git blob bytes differ")?;
        Ok(raw)
    }

    pub fn unchanged(&self, subject: &Subject) -> Result<(), Fault> {
        let held = self.repository(&subject.source.repository)?;
        let identity = serde_json::to_value(held)
            .map_err(|_| Fault::refused("cannot encode source identity"))?;
        let head = self.commit(&subject.source.repository, &subject.source.commit)?;
        Wire::require(
            identity == subject.repository && head.tree.sha == subject.source.tree,
            "source authority moved during observation",
        )
    }
}

impl Tree {
    fn files(self) -> Result<Vec<Entry>, Fault> {
        Wire::require(
            (1..=16384).contains(&self.tree.len()),
            "source tree exceeds bounds",
        )?;
        let mut paths = BTreeSet::new();
        let mut files = Vec::new();
        for entry in self.tree {
            Wire::path(&entry.path)?;
            Wire::hex(&entry.sha)?;
            Wire::require(
                paths.insert(entry.path.clone()),
                "source tree repeats a path",
            )?;
            if entry.kind == "tree" {
                Wire::require(
                    entry.mode == "040000" && entry.size.is_none(),
                    "invalid Git tree entry",
                )?;
                continue;
            }
            Wire::require(entry.kind == "blob", "unresolved source submodule")?;
            if !matches!(entry.mode.as_str(), "100644" | "100755") {
                return Err(Fault::unknown("source has an unsupported nonregular blob"));
            }
            Wire::require(entry.size.is_some(), "source blob size is absent")?;
            files.push(entry);
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Wire::require(!files.is_empty(), "source has no regular files")?;
        Ok(files)
    }
}
