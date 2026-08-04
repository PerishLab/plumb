use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Tar,
    Zip,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tar => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
    pub key: String,
    pub systems: Vec<String>,
    pub archive: String,
    pub format: Format,
    pub runner: String,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub key: String,
    pub file: String,
    pub mime: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Cargo {
    pub registry: String,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Deb {
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub root: PathBuf,
    pub product: String,
    pub authority: String,
    pub binaries: Vec<String>,
    pub target: Vec<Target>,
    pub skill: bool,
    pub cargo: Option<Cargo>,
    pub deb: Option<Deb>,
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
    deb: Option<Deb>,
}
impl Spec {
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let held: Manifest = toml::from_str(&text)
            .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
        let root = path
            .parent()
            .ok_or_else(|| format!("release manifest has no root: {}", path.display()))?;
        let Raw {
            product,
            authority,
            binaries,
            targets,
            skill,
            cargo,
            deb,
        } = held.release;
        let mut spec = Self {
            root: root.to_path_buf(),
            target: targets
                .iter()
                .map(|triple| target(&product, triple))
                .collect::<Result<Vec<_>, _>>()?,
            product,
            authority,
            binaries,
            skill,
            cargo,
            deb,
        };
        if let Some(deb) = &mut spec.deb {
            deb.root = rebase(root, &deb.root);
        }
        spec.validate()?;
        Ok(spec)
    }

    fn validate(&self) -> Result<(), String> {
        let fields = self.product.len() + self.authority.len();
        let binary = fields + self.binaries.len() + self.target.len() > 0;
        let invalid = self.skill || self.deb.is_some() || self.cargo.is_none();
        if !binary && invalid {
            return Err("Cargo-only release must declare only a Cargo attachment".into());
        }
        if binary {
            token("product", &self.product, false)?;
            if !self.authority.starts_with("https://")
                || self.authority.ends_with('/')
                || self.authority.chars().any(char::is_whitespace)
            {
                return Err("authority must be one normalized https URL".into());
            }
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
        if let Some(cargo) = &self.cargo {
            token("Cargo registry", &cargo.registry, false)?;
            if cargo.packages.is_empty() {
                return Err("Cargo attachment must declare ordered packages".into());
            }
            let mut packages = BTreeSet::new();
            for package in &cargo.packages {
                token("Cargo package", package, false)?;
                if !packages.insert(package) {
                    return Err(format!("duplicate Cargo package {package}"));
                }
            }
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

    pub fn assets(&self) -> Vec<Asset> {
        let mut assets = Vec::new();
        if self.skill {
            assets.push(Asset {
                key: "skill".into(),
                file: format!("{}-skill.tar.gz", self.product),
                mime: "application/gzip".into(),
            });
        }
        if self.deb.is_some() {
            assets.push(Asset {
                key: "linux-x64-deb".into(),
                file: format!("{}-x86_64-unknown-linux-gnu.deb", self.product),
                mime: "application/vnd.debian.binary-package".into(),
            });
        }
        assets
    }

    pub fn manifest(&self) -> PathBuf {
        self.root.join("plumb.toml")
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

fn target(product: &str, triple: &str) -> Result<Target, String> {
    let (key, systems, format, runner) = match triple {
        "x86_64-unknown-linux-gnu" => (
            "linux-x64",
            &["Linux:x86_64", "Linux:amd64"][..],
            Format::Tar,
            "linux",
        ),
        "aarch64-apple-darwin" => (
            "darwin-arm64",
            &["Darwin:arm64", "Darwin:aarch64"][..],
            Format::Tar,
            "macos",
        ),
        "x86_64-apple-darwin" => (
            "darwin-x64",
            &["Darwin:x86_64", "Darwin:amd64"][..],
            Format::Tar,
            "macos",
        ),
        "x86_64-pc-windows-msvc" => (
            "windows-x64",
            &["Windows:x86_64", "Windows:amd64"][..],
            Format::Zip,
            "windows",
        ),
        _ => return Err(format!("unsupported release target {triple}")),
    };
    Ok(Target {
        triple: triple.into(),
        key: key.into(),
        systems: systems.iter().map(|system| (*system).into()).collect(),
        archive: format!("{product}-{triple}.{}", format.name()),
        format,
        runner: runner.into(),
    })
}

fn token(subject: &str, value: &str, upper: bool) -> Result<(), String> {
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

fn rebase(root: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        root.join(path)
    } else {
        path.to_path_buf()
    }
}
