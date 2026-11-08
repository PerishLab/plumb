use super::File;
use crate::windtun::qualification::Fault;
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Tree;

impl Tree {
    pub fn object(kind: &str, raw: &[u8]) -> String {
        let mut hash = Sha1::new();
        hash.update(format!("{kind} {}\0", raw.len()));
        hash.update(raw);
        format!("{:x}", hash.finalize())
    }

    pub fn digest(files: &[File]) -> Result<String, Fault> {
        let mut paths = BTreeSet::new();
        let mut nodes = BTreeMap::<String, Vec<(String, String, String)>>::new();
        nodes.insert(String::new(), Vec::new());
        for file in files {
            Self::path(&file.path)?;
            if !paths.insert(file.path.to_ascii_lowercase()) {
                return Err(Fault::refused(
                    "effective source paths have portable aliases",
                ));
            }
            Self::insert(&mut nodes, file)?;
        }
        let mut directories = nodes.keys().cloned().collect::<Vec<_>>();
        directories.sort_by_key(|path| std::cmp::Reverse(path.len()));
        for directory in directories {
            let entries = nodes.remove(&directory).unwrap();
            let digest = Self::entries(entries)?;
            if directory.is_empty() {
                return Ok(digest);
            }
            let (parent, name) = Self::split(&directory);
            nodes
                .get_mut(parent)
                .unwrap()
                .push((name.into(), "40000".into(), digest));
        }
        Err(Fault::refused("effective source tree is absent"))
    }

    fn path(path: &str) -> Result<(), Fault> {
        if !path.is_ascii() {
            return Err(Fault::unknown(
                "effective source portable Unicode paths are not supported",
            ));
        }
        super::super::wire::Wire::require(
            !path.is_empty() && path.len() <= 4096,
            "effective source path exceeds bounds",
        )?;
        super::super::wire::Wire::require(
            !path.contains('\\') && !path.bytes().any(|byte| byte < 32 || byte == 127),
            "effective source path contains nonportable characters",
        )?;
        super::super::wire::Wire::require(
            !path
                .split('/')
                .any(|part| matches!(part, "" | "." | "..") || part.contains(':')),
            "effective source path contains nonportable components",
        )?;
        Ok(())
    }

    fn split(path: &str) -> (&str, &str) {
        path.rsplit_once('/').unwrap_or(("", path))
    }

    fn insert(
        nodes: &mut BTreeMap<String, Vec<(String, String, String)>>,
        file: &File,
    ) -> Result<(), Fault> {
        let (parent, name) = Self::split(&file.path);
        nodes.entry(parent.into()).or_default().push((
            name.into(),
            file.mode.clone(),
            file.oid.clone(),
        ));
        let mut parent = parent;
        while !parent.is_empty() {
            parent = Self::split(parent).0;
            nodes.entry(parent.into()).or_default();
            if nodes.len() > 16384 {
                return Err(Fault::refused(
                    "effective source directory inventory exceeds bounds",
                ));
            }
        }
        Ok(())
    }

    fn entries(mut entries: Vec<(String, String, String)>) -> Result<String, Fault> {
        let mut names = BTreeSet::new();
        let mut aliases = BTreeSet::new();
        for (name, _, _) in &entries {
            if !names.insert(name.clone()) || !aliases.insert(name.to_ascii_lowercase()) {
                return Err(Fault::refused(
                    "effective source repeats a tree entry or file/directory alias",
                ));
            }
        }
        entries.sort_by_cached_key(|(name, mode, _)| {
            if mode == "40000" {
                format!("{name}/")
            } else {
                name.clone()
            }
        });
        let mut raw = Vec::new();
        for (name, mode, oid) in entries {
            raw.extend_from_slice(format!("{mode} {name}\0").as_bytes());
            for offset in (0..40).step_by(2) {
                raw.push(
                    u8::from_str_radix(&oid[offset..offset + 2], 16)
                        .map_err(|_| Fault::refused("effective source tree object is invalid"))?,
                );
            }
        }
        Ok(Self::object("tree", &raw))
    }
}
