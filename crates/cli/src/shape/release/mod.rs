mod attachment;
mod depot;
mod shape;
mod target;

pub use attachment::{Cargo, Cfworker, Chart, Npm, Oci};
pub use depot::Depot;
pub use target::{Format, Target};

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Spec {
    pub root: PathBuf,
    pub product: String,
    pub authority: String,
    pub binaries: Vec<String>,
    pub target: Vec<Target>,
    pub skill: bool,
    pub cargo: Option<Cargo>,
    pub oci: Option<Oci>,
    pub chart: Option<Chart>,
    pub npm: Option<Npm>,
    pub cfworker: Option<Cfworker>,
    pub depot: Option<Depot>,
}

#[derive(Deserialize)]
struct Manifest {
    release: Raw,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Raw {
    product: String,
    authority: String,
    binaries: Vec<String>,
    targets: Vec<String>,
    skill: bool,
    cargo: Option<Cargo>,
    oci: Option<Oci>,
    chart: Option<Chart>,
    npm: Option<Npm>,
    cfworker: Option<Cfworker>,
    depot: Option<Depot>,
}
impl Spec {
    pub(crate) fn controller(root: &Path) -> Result<Self, String> {
        let manifest = root.join("plumb.toml");
        if manifest.is_file()
            && let Ok(spec) = Self::read(&manifest)
            && spec.product == "plumb"
        {
            return Ok(spec);
        }
        Self::resolve(root)
    }

    pub fn resolve(root: &Path) -> Result<Self, String> {
        Self::read(&root.join("plumb.toml"))
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let root = path
            .parent()
            .ok_or_else(|| format!("release manifest has no root: {}", path.display()))?;
        Self::decode(root, &text, &path.display().to_string())
    }

    pub(crate) fn decode(root: &Path, text: &str, subject: &str) -> Result<Self, String> {
        let held: Manifest =
            toml::from_str(text).map_err(|error| format!("cannot parse {subject}: {error}"))?;
        let Raw {
            product,
            authority,
            binaries,
            targets,
            skill,
            cargo,
            oci,
            chart,
            npm,
            cfworker,
            depot,
        } = held.release;
        let spec = Self {
            root: root.to_path_buf(),
            target: targets
                .iter()
                .map(|triple| target::resolve(&product, triple))
                .collect::<Result<Vec<_>, _>>()?,
            product,
            authority,
            binaries,
            skill,
            cargo,
            oci,
            chart,
            npm,
            cfworker,
            depot,
        };
        spec.validate()?;
        Ok(spec)
    }

    fn attachments(&self) -> [Result<(), String>; 5] {
        [
            self.cargo.as_ref().map_or(Ok(()), Cargo::validate),
            self.oci.as_ref().map_or(Ok(()), Oci::validate),
            self.chart
                .as_ref()
                .map_or(Ok(()), |held| held.validate(&self.root)),
            self.npm
                .as_ref()
                .map_or(Ok(()), |held| held.validate(&self.root)),
            self.cfworker
                .as_ref()
                .map_or(Ok(()), |held| held.validate(&self.root)),
        ]
    }

    fn validate(&self) -> Result<(), String> {
        let binary = self.binary();
        if !binary {
            self.standalone()?;
        }
        if binary || !self.product.is_empty() || !self.authority.is_empty() {
            token("product", &self.product, false)?;
            if !self.authority.starts_with("https://")
                || self.authority.ends_with('/')
                || self.authority.chars().any(char::is_whitespace)
            {
                return Err("authority must be one normalized https URL".into());
            }
        }
        if binary {
            if self.binaries.is_empty() {
                return Err("release must declare at least one binary".into());
            }
            let mut binaries = BTreeSet::new();
            for binary in &self.binaries {
                token("binary", binary, false)?;
                if !binaries.insert(binary) {
                    return Err(format!("duplicate binary {binary}"));
                }
            }
            if self.target.is_empty() {
                return Err("release must declare at least one target".into());
            }
            let mut triples = BTreeSet::new();
            for target in &self.target {
                if !triples.insert(&target.triple) {
                    return Err(format!("duplicate target {}", target.triple));
                }
                if target.format == Format::Zip && self.binaries.len() != 1 {
                    return Err("a Windows release currently requires exactly one binary".into());
                }
            }
        }
        if let Some(depot) = &self.depot {
            token("product", &self.product, false)?;
            if self.authority.is_empty() {
                return Err("a depot declaration requires release authority".into());
            }
            depot.validate()?;
        }
        for held in self.attachments() {
            held?;
        }
        Ok(())
    }

    pub fn environment(&self) -> String {
        self.product.to_ascii_uppercase().replace('-', "_")
    }

    pub fn windows(&self) -> Option<&Target> {
        self.target
            .iter()
            .find(|target| target.format == Format::Zip)
    }
}

pub(super) fn token(subject: &str, value: &str, upper: bool) -> Result<(), String> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(format!("{subject} cannot be empty"));
    };
    let head = if upper {
        first.is_ascii_uppercase()
    } else {
        first.is_ascii_lowercase()
    };
    let tail = |held: char| {
        held.is_ascii_digit()
            || if upper {
                held.is_ascii_uppercase() || held == '_'
            } else {
                held.is_ascii_lowercase() || held == '-'
            }
    };
    if !head || !chars.all(tail) {
        return Err(format!("invalid {subject}: {value}"));
    }
    Ok(())
}
