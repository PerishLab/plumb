use super::super::{Spec, token};
use super::{account, host, pair};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component as Segment, Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Oci {
    pub registry: String,
    pub image: Option<String>,
    pub account: String,
    pub binary: Option<String>,
    pub file: Option<String>,
    pub web: Option<String>,
    pub components: BTreeMap<String, Component>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Component {
    pub image: String,
    pub binary: Option<String>,
    pub file: Option<String>,
    pub web: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Binary(String),
    Web(String),
    Source,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub registry: String,
    pub account: String,
    pub image: String,
    pub file: String,
    pub input: Input,
}

impl Oci {
    pub(crate) fn validate(&self, spec: &Spec) -> Result<(), String> {
        host("image registry", &self.registry)?;
        account("image account", &self.account)?;
        if self.components.is_empty() {
            self.single(spec)
        } else {
            self.multi(spec)
        }
    }

    pub fn parts(&self, spec: &Spec) -> Vec<Part> {
        if self.components.is_empty() {
            let input = if let Some(web) = &self.web {
                Input::Web(web.clone())
            } else if let Some(binary) = self.binary.as_deref().or_else(|| spec.primary()) {
                Input::Binary(binary.into())
            } else {
                Input::Source
            };
            return vec![Part {
                name: spec.product.clone(),
                registry: self.registry.clone(),
                account: self.account.clone(),
                image: self.image.clone().unwrap_or_default(),
                file: self.file.clone().unwrap_or_else(|| "Containerfile".into()),
                input,
            }];
        }
        self.components
            .iter()
            .map(|(name, part)| {
                let input = if let Some(web) = &part.web {
                    Input::Web(web.clone())
                } else if let Some(binary) = &part.binary {
                    Input::Binary(binary.clone())
                } else {
                    Input::Source
                };
                Part {
                    name: name.clone(),
                    registry: self.registry.clone(),
                    account: self.account.clone(),
                    image: part.image.clone(),
                    file: part.file.clone().unwrap_or_else(|| "Containerfile".into()),
                    input,
                }
            })
            .collect()
    }

    fn single(&self, spec: &Spec) -> Result<(), String> {
        let Some(image) = self.image.as_deref() else {
            return Err("image attachment must name one owner and one image".into());
        };
        pair(image, "image attachment must name one owner and one image")?;
        entry(
            spec,
            self.binary.as_deref(),
            self.file.as_deref(),
            self.web.as_deref(),
        )
    }

    fn multi(&self, spec: &Spec) -> Result<(), String> {
        let mixed = [&self.image, &self.binary, &self.file, &self.web]
            .into_iter()
            .any(Option::is_some);
        if mixed {
            return Err("cannot mix single and multi-component OCI declarations".into());
        }
        let mut images = BTreeSet::new();
        for (name, part) in &self.components {
            token("image component", name, false)?;
            pair(
                &part.image,
                "image attachment must name one owner and one image",
            )?;
            let lower = part.image.to_ascii_lowercase();
            if !images.insert(lower) {
                return Err(format!("duplicate OCI image: {}", part.image));
            }
            entry(
                spec,
                part.binary.as_deref(),
                part.file.as_deref(),
                part.web.as_deref(),
            )?;
        }
        Ok(())
    }
}

fn entry(
    spec: &Spec,
    binary: Option<&str>,
    file: Option<&str>,
    web: Option<&str>,
) -> Result<(), String> {
    if binary.is_some() && web.is_some() {
        return Err("image attachment cannot declare both binary and web".into());
    }
    if let Some(name) = binary {
        spec.server("image placement", name)?;
    }
    if let Some(dir) = web {
        if file.is_none() {
            return Err("image web attachment requires an explicit file".into());
        }
        directory(&spec.root, dir)?;
    }
    if let Some(path) = file {
        regular(&spec.root, path)?;
    }
    Ok(())
}

fn relative(root: &Path, raw: &str, subject: &str) -> Result<PathBuf, String> {
    if raw.is_empty()
        || raw
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | "..") || part.contains(['\\', ':']))
    {
        return Err(format!(
            "{subject} must name a normalized repository-relative path: {raw}"
        ));
    }
    let mut path = root.to_path_buf();
    for segment in Path::new(raw).components() {
        let Segment::Normal(name) = segment else {
            return Err(format!(
                "{subject} must name a normalized repository-relative path: {raw}"
            ));
        };
        path.push(name);
        if path.is_symlink() {
            return Err(format!("{subject} must not traverse a symlink: {raw}"));
        }
    }
    Ok(path)
}

fn regular(root: &Path, raw: &str) -> Result<(), String> {
    let path = relative(root, raw, "image file")?;
    let meta =
        fs::symlink_metadata(&path).map_err(|_| format!("declared image file is absent: {raw}"))?;
    if !meta.file_type().is_file() {
        return Err(format!("declared image file must be a regular file: {raw}"));
    }
    Ok(())
}

fn directory(root: &Path, raw: &str) -> Result<(), String> {
    let dir = relative(root, raw, "image web")?;
    let meta = fs::symlink_metadata(&dir)
        .map_err(|_| format!("declared image web directory is absent: {raw}"))?;
    if !meta.file_type().is_dir() {
        return Err(format!("declared image web must be a directory: {raw}"));
    }
    let manifest = dir.join("package.json");
    if manifest.is_symlink() {
        return Err(format!(
            "image web package.json must not be a symlink: {raw}/package.json"
        ));
    }
    let info = fs::symlink_metadata(&manifest)
        .map_err(|_| format!("declared image web is missing package.json: {raw}/package.json"))?;
    if !info.file_type().is_file() {
        return Err(format!(
            "image web package.json must be a regular file: {raw}/package.json"
        ));
    }
    let text = fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read {raw}/package.json: {error}"))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid {raw}/package.json: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err(format!("{raw}/package.json must be a JSON object"));
    };
    let named = object
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|held| !held.trim().is_empty());
    if !named {
        return Err(format!(
            "{raw}/package.json must declare a non-empty string name"
        ));
    }
    let built = object
        .get("scripts")
        .and_then(Value::as_object)
        .and_then(|scripts| scripts.get("build"))
        .and_then(Value::as_str)
        .is_some_and(|held| !held.trim().is_empty());
    if !built {
        return Err(format!(
            "{raw}/package.json must declare a non-empty scripts.build"
        ));
    }
    Ok(())
}
