use super::super::wharf::{git, text};
use super::Seat;
use super::decision::{Decision, Result, require};
use super::native::{Native, content};
use crate::shape::release::Spec;
use plumb::windtun::qualification::Read;
use serde_json::Value;
use std::collections::BTreeMap;

fn name(url: &str) -> Result<&str> {
    let path = [
        "https://github.com/",
        "git@github.com:",
        "ssh://git@github.com/",
    ]
    .iter()
    .find_map(|p| url.strip_prefix(p))
    .unwrap_or("");
    let path = path.strip_suffix(".git").unwrap_or(path);
    let valid = |part: &str| {
        let named = !part.is_empty() && !part.starts_with('.');
        named
            && !part.ends_with(".git")
            && part
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    };
    require(
        path.split_once('/')
            .is_some_and(|(owner, repo)| valid(owner) && valid(repo)),
        "expected one canonical GitHub repository URL",
    )?;
    Ok(path)
}

pub(super) fn repository(seat: &Seat<'_>) -> Result<(String, String)> {
    let get = |args: &[&str]| text("read successor remote", git(seat.root, args)?);
    let fetch = get(&["remote", "get-url", "--all", seat.remote])?;
    let push = get(&["remote", "get-url", "--push", "--all", seat.remote])?;
    let repo = name(&fetch)?;
    require(
        name(&push)? == repo,
        "fetch and push repository identities differ",
    )?;
    let rules = git(
        seat.root,
        &[
            "config",
            "--null",
            "--get-regexp",
            r"^url\..*\.(insteadof|pushinsteadof)$",
        ],
    )?;
    match rules.status.code() {
        Some(1) => {}
        Some(0) => {
            let text = std::str::from_utf8(&rules.stdout)
                .map_err(|_| "specified successor: unreadable Git transport rule")?;
            for row in text.split_terminator('\0') {
                let (_, prefix) = row
                    .split_once('\n')
                    .ok_or("specified successor: invalid Git transport rule")?;
                require(
                    !push.starts_with(prefix),
                    "qualified Git destination would be rewritten",
                )?;
            }
        }
        _ => return Err("specified successor: Git transport rules unreadable".into()),
    }
    Ok((repo.to_string(), push))
}

impl<R: Read> Native<'_, R> {
    pub fn manifest(&self, id: u64, commit: &str) -> Result<Vec<u8>> {
        let path = "plumb.toml";
        let file = self.api(&format!("repositories/{id}/contents/{path}?ref={commit}"))?;
        content(&file, path, file["sha"].as_str().unwrap_or(""))
    }

    pub fn manifests(&self, record: &Decision, seat: &Seat<'_>) -> Result<()> {
        let old = self.manifest(record.identity, &record.predecessor.commit)?;
        let current = self.manifest(record.identity, &record.successor.commit)?;
        let local = std::fs::read(seat.root.join("plumb.toml")).map_err(|e| e.to_string())?;
        require(
            current == local,
            "local release declaration differs from approved candidate",
        )?;
        let spec = Spec::decode(
            seat.root,
            std::str::from_utf8(&old).map_err(|e| e.to_string())?,
            "original native plumb.toml",
        )?;
        require(
            spec.authority == record.authority && seat.spec.authority == record.authority,
            "original release authority differs",
        )?;
        require(
            spec.product == seat.spec.product
                && spec.skill == seat.spec.skill
                && spec.source() == seat.spec.source(),
            "original product or applicable obligations differ",
        )
    }

    pub fn refs(&self, id: u64, prefix: &str) -> Result<BTreeMap<String, String>> {
        let val = self.api(&format!("repositories/{id}/git/matching-refs/{prefix}"))?;
        let mut seen = BTreeMap::new();
        for r in val
            .as_array()
            .ok_or("specified successor: invalid native ref listing")?
        {
            let key = r["ref"].as_str().ok_or("native ref missing name")?;
            let sha = r["object"]["sha"]
                .as_str()
                .ok_or("native ref missing object")?;
            require(
                key.starts_with(&format!("refs/{prefix}"))
                    && super::decision::hex(sha, 40)
                    && seen.insert(key.into(), sha.into()).is_none(),
                "unexpected or repeated native ref",
            )?;
        }
        Ok(seen)
    }

    pub fn history(&self, record: &Decision) -> Result<()> {
        let id = record.identity;
        let old = &record.predecessor;
        let tag = self.immutable(id, &old.marker)?;
        require(
            tag["sha"] == old.tag && tag["object"]["sha"] == old.commit,
            "original native marker or commit changed",
        )?;
        for (commit, tree) in [
            (&old.commit, &old.tree),
            (&record.successor.commit, &record.successor.tree),
        ] {
            let v = self.api(&format!("repositories/{id}/git/commits/{commit}"))?;
            require(
                v["sha"] == *commit && v["tree"]["sha"] == *tree,
                "native source tree differs",
            )?;
        }
        let ancestry = self.api(&format!(
            "repositories/{id}/compare/{}...main?per_page=1",
            old.commit
        ))?;
        require(
            ancestry["merge_base_commit"]["sha"] == old.commit
                && matches!(ancestry["status"].as_str(), Some("ahead" | "identical")),
            "native main does not retain the original commit",
        )
    }
}

pub(super) fn annotated(tag: &Value, marker: &str, record: &Decision) -> Result<()> {
    let expected = if marker == record.successor.rc {
        marker.to_string()
    } else {
        format!("{marker}\n\npromotes {}", record.successor.rc)
    };
    require(
        tag["object"]["sha"] == record.successor.commit
            && tag["message"]
                .as_str()
                .is_some_and(|s| s.trim_end_matches('\n') == expected),
        "annotated marker source or promotion differs",
    )
}
