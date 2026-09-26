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
    pub account: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Chart {
    pub registry: String,
    pub chart: String,
    pub account: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Cfworker {
    pub account: String,
    pub domain: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Npm {
    pub registry: String,
    pub packages: Vec<String>,
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
        account("image account", &self.account)?;
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
        account("chart account", &self.account)?;
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
        if self.packages.is_empty() {
            return Err("module attachment must declare ordered packages".into());
        }
        let mut packages = BTreeSet::new();
        for package in &self.packages {
            let bare = package.rsplit('/').next().unwrap_or_default();
            if bare.is_empty() {
                return Err("module attachment must name one package".into());
            }
            if !packages.insert(package) {
                return Err(format!("duplicate module package {package}"));
            }
            if !root.join("packages").join(bare).is_dir() {
                return Err(format!("declared module root is absent: packages/{bare}"));
            }
        }
        Ok(())
    }
}

impl Cfworker {
    pub(super) fn validate(&self, root: &Path) -> Result<(), String> {
        account("worker account", &self.account)?;
        hostname("worker domain", &self.domain)?;
        if !root.join("apps").is_dir() {
            return Err("declared worker root is absent: apps".into());
        }
        Ok(())
    }
}

fn hostname(subject: &str, value: &str) -> Result<(), String> {
    let labels: Vec<&str> = value.split('.').collect();
    if labels.len() < 2 {
        return Err(format!("{subject} must be one fully qualified hostname"));
    }
    for held in labels {
        if !label(held) {
            return Err(format!(
                "{subject} label {held} is not a valid RFC 1123 label"
            ));
        }
    }
    Ok(())
}

fn label(value: &str) -> bool {
    let sized = (1..=63).contains(&value.len());
    let shaped = value
        .chars()
        .all(|held| held.is_ascii_lowercase() || held.is_ascii_digit() || held == '-');
    let edged = value.starts_with('-') || value.ends_with('-');
    sized && shaped && !edged
}

fn pair(value: &str, wrong: &str) -> Result<String, String> {
    let seats: Vec<&str> = value.split('/').collect();
    if seats.len() < 2 || seats.iter().any(|held| held.is_empty()) {
        return Err(wrong.into());
    }
    Ok(seats[seats.len() - 1].to_string())
}

fn host(subject: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('/') || value.chars().any(char::is_whitespace) {
        return Err(format!("{subject} must be one bare host"));
    }
    Ok(())
}

const MARKS: [char; 3] = ['-', '_', '.'];

fn account(subject: &str, value: &str) -> Result<(), String> {
    let shaped = value
        .chars()
        .all(|held| held.is_ascii_alphanumeric() || MARKS.contains(&held));
    if shaped && !value.is_empty() {
        Ok(())
    } else {
        Err(format!("{subject} must be one forge account name: {value}"))
    }
}
