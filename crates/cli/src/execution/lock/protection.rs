use super::{Scope, reference, review::Declaration};
use plumb::config::Execution;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct Protection {
    scope: Scope,
    policy: String,
}

impl Protection {
    pub(crate) fn read<'a>(
        root: &Path,
        guard: bool,
        actions: impl Iterator<Item = &'a str>,
        resolution: Option<plumb::packages::Resolution>,
    ) -> Result<Option<Self>, String> {
        if !guard {
            return Ok(None);
        }
        let Some(declaration) = Declaration::enforced(root)? else {
            return Ok(None);
        };
        let actions = actions.collect::<Vec<_>>();
        if !["guard/rust", "guard/ectropy"]
            .iter()
            .all(|name| actions.contains(name))
        {
            return Err(
                "lock protection requires both guard/rust and guard/ectropy actions".into(),
            );
        }
        let policy = Self::policy(root)?;
        let tree = plumb::guard::tree(root)?;
        let bytes = serde_json::to_vec(&serde_json::json!({"schema":"plumb.lock-protection/v1", "tree":tree, "declaration":declaration, "ectropy":policy})).map_err(|error| error.to_string())?;
        Ok(Some(Self {
            policy: format!("{:x}", Sha256::digest(bytes)),
            scope: Scope {
                all: false,
                source: tree.clone(),
                tree,
                resolution,
            },
        }))
    }

    pub(crate) fn bind(&self, name: &str, input: &str) -> String {
        if !matches!(name, "guard/lock" | "guard/ectropy") {
            return input.into();
        }
        let bytes = serde_json::to_vec(&(input, &self.policy)).expect("lock policy serializes");
        format!("{:x}", Sha256::digest(bytes))
    }

    pub(crate) fn clippy(&self, execution: &Execution) -> Result<String, String> {
        let argv = ["cargo", "clippy", "--version"].map(str::to_owned).to_vec();
        let output = execution.output(&argv)?;
        if !output.status.success() {
            return Err("cannot identify Clippy for lock protection".into());
        }
        let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
        if stdout.trim().is_empty() {
            return Err("Clippy identity for lock protection is empty".into());
        }
        Ok(stdout.replace("\r\n", "\n").trim_end_matches('\n').into())
    }

    pub(crate) fn run(&self, root: &Path, execution: Option<&Execution>) -> Result<(), String> {
        let execution = execution.ok_or("lock protection needs managed Cargo execution")?;
        super::check(root, execution, self.scope.clone())?;
        Ok(())
    }

    fn policy(root: &Path) -> Result<String, String> {
        let (_, digest) = reference::file(root, "ectropy.toml")?;
        let text = std::fs::read_to_string(root.join("ectropy.toml"))
            .map_err(|error| error.to_string())?;
        let document: toml::Table = text
            .parse()
            .map_err(|error| format!("invalid lock syntax policy: {error}"))?;
        let mut found = false;
        for (name, paths) in ["grant", "ban"].into_iter().flat_map(|name| {
            document
                .get(name)
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .map(move |entry| (name, entry))
        }) {
            if paths.get("syntax").and_then(toml::Value::as_str) != Some("lock") {
                continue;
            }
            let paths = paths
                .get("paths")
                .and_then(toml::Value::as_array)
                .filter(|paths| !paths.is_empty())
                .ok_or("lock syntax policy needs explicit paths")?;
            for path in paths {
                let path = path.as_str().ok_or("lock syntax path is not a string")?;
                if name == "grant" {
                    Self::grant(root, path)?;
                }
            }
            found = true;
        }
        if !found {
            return Err("lock protection requires explicit Ectropy lock grant/ban policy".into());
        }
        Ok(digest)
    }

    fn grant(root: &Path, path: &str) -> Result<(), String> {
        let (name, _) = reference::file(root, path)?;
        if name != path || path.contains(['*', '?', '[', ']', '\\']) {
            return Err(
                "lock grants require exact tracked regular files, never path-wide exemptions"
                    .into(),
            );
        }
        Ok(())
    }
}
