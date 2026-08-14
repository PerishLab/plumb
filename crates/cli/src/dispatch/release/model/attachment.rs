use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Cargo {
    pub registry: String,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Oci {
    pub registry: String,
    pub image: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Chart {
    pub registry: String,
    pub chart: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Npm {
    pub registry: String,
    pub package: String,
}

impl Cargo {
    pub(super) fn validate(&self) -> Result<(), String> {
        super::token("Cargo registry", &self.registry, false)?;
        if self.packages.is_empty() {
            return Err("Cargo attachment must declare ordered packages".into());
        }
        let mut packages = BTreeSet::new();
        for package in &self.packages {
            super::token("Cargo package", package, false)?;
            if !packages.insert(package) {
                return Err(format!("duplicate Cargo package {package}"));
            }
        }
        Ok(())
    }
}

impl Oci {
    pub(super) fn validate(&self) -> Result<(), String> {
        host("image registry", &self.registry)?;
        pair(
            &self.image,
            "image attachment must name one owner and one image",
        )
        .map(|_| ())
    }
}

impl Chart {
    pub(super) fn validate(&self, root: &Path) -> Result<(), String> {
        host("chart registry", &self.registry)?;
        let name = pair(
            &self.chart,
            "chart attachment must name one owner and one chart",
        )?;
        if !root.join("charts").join(&name).is_dir() {
            return Err(format!("declared chart root is absent: charts/{name}"));
        }
        Ok(())
    }
}

impl Npm {
    pub(super) fn validate(&self, root: &Path) -> Result<(), String> {
        if !self.registry.starts_with("https://") {
            return Err("module registry must be one https URL".into());
        }
        let bare = self.package.rsplit('/').next().unwrap_or_default();
        if bare.is_empty() {
            return Err("module attachment must name one package".into());
        }
        if !root.join("packages").join(bare).is_dir() {
            return Err(format!("declared module root is absent: packages/{bare}"));
        }
        Ok(())
    }
}

fn pair(value: &str, wrong: &str) -> Result<String, String> {
    let mut seats = value.split('/');
    let owner = seats.next().unwrap_or_default();
    let name = seats.next().unwrap_or_default();
    if seats.next().is_some() || owner.is_empty() || name.is_empty() {
        return Err(wrong.into());
    }
    Ok(name.to_string())
}

fn host(subject: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('/') || value.chars().any(char::is_whitespace) {
        return Err(format!("{subject} must be one bare host"));
    }
    Ok(())
}
