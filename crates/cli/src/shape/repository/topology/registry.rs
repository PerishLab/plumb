use super::model::{Declared, Edge, Node};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub(super) const SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Candidate {
    pub version: String,
    pub checksum: String,
    pub yanked: bool,
}

#[derive(Clone, Deserialize)]
pub(super) struct Entry {
    pub name: String,
    pub vers: String,
    pub cksum: String,
    pub yanked: bool,
    pub deps: Vec<Dependency>,
    pub v: Option<u32>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Dependency {
    pub name: String,
    pub package: Option<String>,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    #[serde(rename = "default_features")]
    pub default: bool,
    pub target: Option<String>,
    pub kind: String,
    pub registry: Option<String>,
}

impl Entry {
    pub fn qualify(&self, locked: &BTreeMap<(String, String), String>) -> Result<(), String> {
        if let Some(checksum) = locked.get(&(self.name.clone(), self.vers.clone()))
            && checksum != &self.cksum
        {
            return Err(format!(
                "canonical index differs from locked checksum for {} {}",
                self.name, self.vers
            ));
        }
        Ok(())
    }

    pub fn read(name: &str, bytes: &[u8]) -> Result<Vec<Self>, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "registry index is not UTF-8")?;
        let mut entries = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for line in text.lines().filter(|line| !line.is_empty()) {
            let entry: Self = serde_json::from_str(line)
                .map_err(|error| format!("invalid index for {name}: {error}"))?;
            if entry.v.is_some_and(|version| ![1, 2].contains(&version)) {
                return Err("unsupported index schema".into());
            }
            let version = semver::Version::parse(&entry.vers).map_err(|error| error.to_string())?;
            let valid = entry.name == name
                && seen.insert(version)
                && super::evidence::checksum(&entry.cksum);
            if !valid {
                return Err(format!("unqualified index identity for {name}"));
            }
            entries.push(entry);
        }
        if entries.is_empty() {
            return Err(format!("empty registry index for {name}"));
        }
        Ok(entries)
    }

    pub fn candidate(&self) -> Candidate {
        Candidate {
            version: self.vers.clone(),
            checksum: self.cksum.clone(),
            yanked: self.yanked,
        }
    }
}

impl Dependency {
    pub fn declared(&self) -> Declared {
        Declared {
            name: self.package.as_ref().unwrap_or(&self.name).clone(),
            rename: self.package.as_ref().map(|_| self.name.clone()),
            source: Some(
                self.registry
                    .as_deref()
                    .map(authority)
                    .unwrap_or(SOURCE)
                    .into(),
            ),
            req: self.req.clone(),
            path: None,
            kind: (self.kind != "normal").then(|| self.kind.clone()),
            target: self.target.clone(),
            optional: self.optional,
            features: self.features.clone(),
            default: self.default,
        }
    }

    pub fn edge(&self, from: &str) -> Edge {
        let declared = self.declared();
        Edge {
            from: from.into(),
            to: cohort(&declared.name, declared.source.as_deref()),
            alias: declared.alias(),
            requirement: declared.req,
            source: declared.source,
            kind: self.kind.clone(),
            target: self.target.clone(),
            optional: self.optional,
            features: self.features.clone(),
            default: self.default,
            versions: Vec::new(),
            resolved: false,
            propagated: self.kind != "dev",
            seat: Path::new("registry-index").join(&self.name),
        }
    }
}

pub(super) fn cohort(name: &str, source: Option<&str>) -> String {
    format!("candidates:{name}:{source:?}")
}

pub(super) fn node(name: &str, source: Option<&str>) -> Node {
    Node {
        id: cohort(name, source),
        name: name.into(),
        source: source.map(str::to_owned),
        version: None,
        manifest: None,
        domain: source == Some(crate::catalog::set::CARGO.index),
        workspace: false,
        features: Vec::new(),
        aliases: Vec::new(),
        candidates: Vec::new(),
    }
}

fn authority(source: &str) -> &str {
    let domain = crate::catalog::set::CARGO.index;
    if source == "https://github.com/rust-lang/crates.io-index" {
        return SOURCE;
    }
    if source == domain
        || format!("registry+{source}") == domain
        || format!("sparse+{source}") == domain
    {
        return domain;
    }
    source
}

pub(super) fn qualified(package: &str) -> Result<(), String> {
    if package.is_empty()
        || package.len() > 64
        || !package
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err("registry package has no qualified package".into());
    }
    Ok(())
}

pub(super) fn url(package: &str) -> String {
    let name = package.to_ascii_lowercase();
    let path = match name.len() {
        1 => name.clone(),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    };
    format!("https://index.crates.io/{path}")
}

pub(super) fn fetch(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("cannot read declaration index: {error}"))?;
    if response.status().as_u16() != 200 || response.headers().contains_key("Content-Range") {
        return Err("partial declaration index response".into());
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("declaration index exceeds byte budget".into());
    }
    Ok(bytes)
}

pub(super) fn unmodified(root: &Path) -> Result<(), String> {
    let manifest =
        fs::read_to_string(root.join("Cargo.toml")).map_err(|error| error.to_string())?;
    inspect(&manifest)?;
    for ancestor in root.ancestors() {
        for name in ["config", "config.toml"] {
            let path = ancestor.join(".cargo").join(name);
            if path.exists() {
                inspect(&fs::read_to_string(path).map_err(|error| error.to_string())?)?;
            }
        }
    }
    let home = plumb::config::value("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            plumb::config::value(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| Path::new(&home).join(".cargo"))
        });
    if let Some(home) = home {
        for name in ["config", "config.toml"] {
            let path = Path::new(&home).join(name);
            if path.exists() {
                inspect(&fs::read_to_string(path).map_err(|error| error.to_string())?)?;
            }
        }
    }
    for (name, _) in std::env::vars_os() {
        let name = name.to_string_lossy();
        let registry =
            ["CARGO_REGISTRIES_CRATES_IO_INDEX", "CARGO_REGISTRY_INDEX"].contains(&name.as_ref());
        if name.starts_with("CARGO_SOURCE_") || name.starts_with("CARGO_PATCH_") || registry {
            return Err("declaration closure has unqualified environment overrides".into());
        }
    }
    Ok(())
}

fn inspect(text: &str) -> Result<(), String> {
    let table: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
    if table
        .get("registries")
        .and_then(|value| value.get("crates-io"))
        .and_then(|value| value.get("index"))
        .is_some()
    {
        return Err("declaration closure has a crates.io registry override".into());
    }
    if ["patch", "replace", "source"]
        .iter()
        .any(|name| table.contains_key(*name))
    {
        return Err(
            "declaration closure cannot qualify patch, replacement or source overrides".into(),
        );
    }
    Ok(())
}
