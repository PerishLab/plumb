use super::decision::{Decision, Result, hex, require};
use base64::{Engine, engine::general_purpose::STANDARD};
use plumb::windtun::qualification::Read;
use serde_json::Value;
use sha1::{Digest, Sha1};

const ORGANIZATION: u64 = 1_388_431_417;
const ORG: &str = "PerishLab/.github";
const LIMIT: usize = 1024 * 1024;

pub(super) struct Native<'a, R>(pub &'a R);

impl<R: Read> Native<'_, R> {
    pub fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.0
            .bytes(url, LIMIT)
            .map_err(|e| format!("specified successor: {e}"))
    }

    pub fn api(&self, path: &str) -> Result<Value> {
        let raw = self.bytes(&format!("https://api.github.com/{path}"))?;
        serde_json::from_slice(&raw)
            .map_err(|e| format!("specified successor: invalid native response: {e}"))
    }

    pub fn product(&self, repo: &str) -> Result<u64> {
        let v = self.api(&format!("repos/{repo}"))?;
        require(
            v["full_name"] == repo,
            "native product repository name differs",
        )?;
        v["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or_else(|| "specified successor: missing native repository identity".into())
    }

    pub fn main(&self) -> Result<String> {
        let repo = self.api(&format!("repos/{ORG}"))?;
        require(
            repo["id"] == ORGANIZATION
                && repo["full_name"] == ORG
                && repo["default_branch"] == "main",
            "organization repository identity differs",
        )?;
        require(
            repo["node_id"] == "R_kgDOUsHIOQ",
            "organization repository node differs",
        )?;
        let branch = self.api(&format!("repos/{ORG}/branches/main"))?;
        let head = branch["commit"]["sha"].as_str().unwrap_or("");
        require(
            branch["name"] == "main" && branch["protected"] == true && hex(head, 40),
            "organization main is missing or unprotected",
        )?;
        Ok(head.to_string())
    }

    pub fn decision(&self, id: u64, version: &str, blob: &str) -> Result<(String, Decision)> {
        require(hex(blob, 40), "expected exact lowercase Git blob id")?;
        let head = self.main()?;
        let path = format!("governance/release-successors/{id}/{version}.json");
        let value = self.api(&format!("repos/{ORG}/contents/{path}?ref={head}"))?;
        let bytes = content(&value, &path, blob)?;
        let decision = serde_json::from_slice(&bytes)
            .map_err(|e| format!("specified successor: invalid decision: {e}"))?;
        Ok((head, decision))
    }

    pub fn immutable(&self, repo: &str, name: &str) -> Result<Value> {
        let reference = format!("refs/tags/{name}");
        let value = self.api(&format!("repos/{repo}/git/ref/tags/{name}"))?;
        let object = value["object"]["sha"].as_str().unwrap_or("");
        require(
            value["ref"] == reference && value["object"]["type"] == "tag" && hex(object, 40),
            "native marker must be an annotated tag",
        )?;
        let tag = self.api(&format!("repos/{repo}/git/tags/{object}"))?;
        require(
            tag["sha"] == object && tag["tag"] == name && tag["object"]["type"] == "commit",
            "native tag object or target type differs",
        )?;
        Ok(tag)
    }

    pub fn branch(&self, repo: &str, branch: &str, commit: &str) -> Result<()> {
        let v = self.api(&format!("repos/{repo}/git/ref/heads/{branch}"))?;
        require(
            v["ref"] == format!("refs/heads/{branch}")
                && v["object"]["type"] == "commit"
                && v["object"]["sha"] == commit,
            "native branch differs from the approved source",
        )
    }
}

pub(super) fn content(value: &Value, path: &str, blob: &str) -> Result<Vec<u8>> {
    require(
        value["type"] == "file" && value["path"] == path,
        "organization record path or file type differs",
    )?;
    require(
        value["sha"] == blob && value["encoding"] == "base64",
        "organization record blob or encoding differs",
    )?;
    let encoded = value["content"].as_str().unwrap_or("");
    require(
        !encoded.is_empty() && encoded.len() <= 32 * 1024,
        "organization record exceeds bounds",
    )?;
    let raw = STANDARD
        .decode(encoded.lines().collect::<String>())
        .map_err(|e| e.to_string())?;
    let mut git = Sha1::new();
    git.update(format!("blob {}\0", raw.len()));
    git.update(&raw);
    require(
        value["size"].as_u64() == Some(raw.len() as u64) && format!("{:x}", git.finalize()) == blob,
        "organization Git blob bytes differ",
    )?;
    Ok(raw)
}
