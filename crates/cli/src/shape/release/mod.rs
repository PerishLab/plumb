mod attachment;
mod depot;
mod retire;
mod shape;
mod target;

pub use attachment::{Cargo, Cfworker, Chart, Deb, Npm, Oci};
pub use depot::Depot;
pub use retire::Retire;
pub use target::{Format, Target};

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Spec {
    pub root: PathBuf,
    pub configuration: Option<String>,
    pub profile: Option<String>,
    pub product: String,
    pub authority: String,
    pub route: Option<String>,
    pub derivatives: Vec<plumb::depot::v3::Kind>,
    pub binaries: Vec<String>,
    pub target: Vec<Target>,
    pub skill: bool,
    pub cargo: Option<Cargo>,
    pub oci: Option<Oci>,
    pub chart: Option<Chart>,
    pub npm: Option<Npm>,
    pub cfworker: Option<Cfworker>,
    pub depends: std::collections::BTreeMap<String, Vec<PathBuf>>,
    pub deb: Option<Deb>,
    pub retire: Option<Retire>,
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
    depends: std::collections::BTreeMap<String, Vec<PathBuf>>,
    deb: Option<Deb>,
    retire: Option<Retire>,
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
        let Some(target) = super::product::governance(root)? else {
            return Self::read(&root.join("plumb.toml"));
        };
        Self::governed(root, target)
    }

    pub(crate) fn governed(root: &Path, target: super::product::Target) -> Result<Self, String> {
        let route = target.depot.clone();
        let derivatives = target.derivatives().to_vec();
        let Some(profile) = target.profile else {
            return Self::read(&root.join("plumb.toml"));
        };
        let subject = format!("product profile {}", profile.digest);
        let mut spec = Self::decode(root, &profile.manifest, &subject)?;
        if spec.product != target.product || spec.authority != target.authority {
            return Err(format!(
                "{subject} release identity differs from its product definition"
            ));
        }
        spec.configuration = Some(profile.configuration);
        spec.profile = Some(profile.digest);
        spec.route = Some(route);
        spec.derivatives = derivatives;
        Ok(spec)
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
            depends,
            deb,
            retire,
            depot,
        } = held.release;
        let mut spec = Self {
            root: root.to_path_buf(),
            configuration: None,
            profile: None,
            target: targets
                .iter()
                .map(|triple| target::resolve(&product, triple))
                .collect::<Result<Vec<_>, _>>()?,
            product,
            authority,
            route: depot.as_ref().map(|held| held.source.clone()),
            derivatives: depot
                .as_ref()
                .map_or_else(Vec::new, |held| held.derivatives.clone()),
            binaries,
            skill,
            cargo,
            oci,
            chart,
            npm,
            cfworker,
            depends,
            deb,
            retire,
            depot,
        };
        if let Some(deb) = &mut spec.deb
            && deb.root.is_relative()
        {
            deb.root = root.join(&deb.root);
        }
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
        if self.skill && !self.root.join("skills").join(&self.product).is_dir() {
            return Err(format!(
                "declared skill root is absent: {}",
                self.root.join("skills").join(&self.product).display()
            ));
        }
        if let Some(retire) = &self.retire {
            retire.validate()?;
        }
        if let Some(depot) = &self.depot {
            token("product", &self.product, false)?;
            if self.authority.is_empty() {
                return Err("a depot declaration requires release authority".into());
            }
            depot.validate(&self.binaries)?;
        }
        for held in self.attachments() {
            held?;
        }
        if let Some(deb) = &self.deb {
            if !self
                .target
                .iter()
                .any(|target| target.triple == "x86_64-unknown-linux-gnu")
            {
                return Err("Debian attachment requires x86_64-unknown-linux-gnu".into());
            }
            if !deb.root.join("control").is_file() {
                return Err(format!(
                    "Debian attachment has no control template: {}",
                    deb.root.join("control").display()
                ));
            }
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

    pub fn target(&self, triple: &str) -> Result<&Target, String> {
        self.target
            .iter()
            .find(|target| target.triple == triple)
            .ok_or_else(|| format!("release does not declare target {triple}"))
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
