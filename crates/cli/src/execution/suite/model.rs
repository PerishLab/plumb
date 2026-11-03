use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct Plan {
    pub programs: Vec<Program>,
    pub docs: bool,
    pub fallback: Vec<String>,
}

#[derive(Debug)]
pub(super) struct Program {
    pub package: String,
    pub name: String,
    pub target: Target,
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub features: BTreeSet<String>,
    pub custom: bool,
}

#[derive(Deserialize)]
pub(super) struct Metadata {
    pub packages: Vec<Package>,
    #[serde(rename = "workspace_members")]
    pub members: BTreeSet<String>,
    #[serde(rename = "workspace_default_members")]
    pub defaults: BTreeSet<String>,
    #[serde(rename = "workspace_root")]
    pub root: PathBuf,
    #[serde(rename = "target_directory")]
    pub output: PathBuf,
}

#[derive(Deserialize)]
pub(super) struct Package {
    pub id: String,
    pub name: String,
    #[serde(rename = "manifest_path")]
    pub manifest: PathBuf,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct Target {
    pub name: String,
    pub kind: Vec<String>,
    #[serde(rename = "src_path")]
    pub source: PathBuf,
    pub test: bool,
    pub doctest: bool,
    #[serde(default, rename = "required-features")]
    pub required: BTreeSet<String>,
}

impl Target {
    pub fn unit(&self) -> bool {
        self.library() || self.kind == ["bin"]
    }

    pub fn library(&self) -> bool {
        !self.kind.is_empty()
            && self.kind.iter().all(|kind| {
                matches!(
                    kind.as_str(),
                    "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro"
                )
            })
    }

    pub fn role(&self) -> Result<&str, String> {
        if self.library() {
            return Ok("lib");
        }
        match self.kind.as_slice() {
            [kind] if matches!(kind.as_str(), "bin" | "test" | "example") => Ok(kind),
            _ => Err(format!("unsupported target kind for {}", self.name)),
        }
    }

    pub fn agrees(&self, other: &Self) -> bool {
        self.identity() == other.identity()
            && self.test == other.test
            && self.doctest == other.doctest
    }

    fn identity(&self) -> (&str, &[String], &Path) {
        (&self.name, &self.kind, &self.source)
    }
}

#[derive(Deserialize)]
pub(super) struct Artifact {
    #[serde(rename = "package_id")]
    pub package: String,
    pub target: Target,
    pub profile: Profile,
    pub executable: Option<PathBuf>,
    pub features: BTreeSet<String>,
    #[serde(default)]
    pub fresh: bool,
}

impl Artifact {
    pub fn same(&self, other: &Self) -> bool {
        (&self.package, &self.features, &self.executable)
            == (&other.package, &other.features, &other.executable)
            && self.profile.test == other.profile.test
            && self.target.agrees(&other.target)
    }
}

#[derive(Deserialize)]
pub(super) struct Profile {
    pub test: bool,
}

pub(super) fn contained(path: &Path, root: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!(
            "Cargo reported a relative path: {}",
            path.display()
        ));
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve Cargo path {}: {error}", path.display()))?;
    if !path.starts_with(root) {
        return Err(format!(
            "Cargo path leaves its admitted root: {}",
            path.display()
        ));
    }
    Ok(path)
}

pub(super) struct Group<'a> {
    pub arguments: Vec<String>,
    pub programs: Vec<&'a Program>,
}

impl Plan {
    pub fn groups(&self, metadata: &str) -> Result<Vec<Group<'_>>, String> {
        let metadata: Metadata = serde_json::from_str(metadata)
            .map_err(|error| format!("invalid Cargo metadata: {error}"))?;
        let mut groups = BTreeMap::<Vec<String>, Vec<&Program>>::new();
        for program in &self.programs {
            groups
                .entry(selector(&program.target)?)
                .or_default()
                .push(program);
        }
        for package in &metadata.packages {
            if !metadata.defaults.contains(&package.id) {
                continue;
            }
            for target in &package.targets {
                let Ok(arguments) = selector(target) else {
                    continue;
                };
                let Some(programs) = groups.get(&arguments) else {
                    continue;
                };
                if !programs
                    .iter()
                    .any(|program| program.package == package.id && program.target.agrees(target))
                {
                    return Err(format!(
                        "selector {} also selects unqualified target {}:{}",
                        arguments.join(" "),
                        package.name,
                        target.name
                    ));
                }
            }
        }
        if groups.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![Group {
            arguments: groups.into_keys().flatten().collect(),
            programs: self.programs.iter().collect(),
        }])
    }
}

fn selector(target: &Target) -> Result<Vec<String>, String> {
    let role = target.role()?;
    if role == "lib" {
        return Ok(vec!["--lib".into()]);
    }
    if target.name.is_empty()
        || !target
            .name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!(
            "target name is not an exact Cargo selector: {}",
            target.name
        ));
    }
    Ok(vec![format!("--{role}"), target.name.clone()])
}
