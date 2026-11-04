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

#[derive(Debug)]
pub(super) struct Schedule {
    pub cores: usize,
    pub slots: usize,
    pub order: Vec<usize>,
    pub grants: Vec<usize>,
}

impl Schedule {
    pub fn cold(cores: usize, count: usize) -> Self {
        Self {
            cores: cores.max(1),
            slots: count.min(1),
            order: (0..count).collect(),
            grants: vec![cores.max(1); count],
        }
    }

    pub fn weighted(cores: usize, work: &[u64]) -> Self {
        let cores = cores.max(1);
        let count = work.len();
        let slots = count.min(cores).min(cores.isqrt().max(2));
        let mut schedule = Self {
            cores,
            slots,
            order: (0..count).collect(),
            grants: vec![cores / slots.max(1); count],
        };
        schedule
            .order
            .sort_by_key(|index| (std::cmp::Reverse(work[*index]), *index));
        let total = work
            .iter()
            .map(|value| u128::from(*value).max(1))
            .sum::<u128>();
        if let Some(&index) = schedule.order.first() {
            let largest = u128::from(work[index]).max(1);
            if count > 1 && cores > 1 && largest > total / 2 {
                let grant = (largest * cores as u128).div_ceil(total) as usize;
                let grant = grant.clamp(1, cores - 1);
                schedule.grants.fill(cores - grant);
                schedule.grants[index] = grant;
            }
        }
        schedule
    }

    pub fn validate(&self) -> Result<(), String> {
        let expected = (0..self.grants.len()).collect::<BTreeSet<_>>();
        let actual = self.order.iter().copied().collect::<BTreeSet<_>>();
        if self.order.len() != self.grants.len() || expected != actual {
            return Err("test schedule has an invalid program selection".into());
        }
        if self.cores == 0 || (!self.order.is_empty() && self.slots == 0) {
            return Err("test schedule has an empty execution budget".into());
        }
        if self
            .grants
            .iter()
            .any(|grant| *grant == 0 || *grant > self.cores)
        {
            return Err("test schedule has an invalid thread allocation".into());
        }
        Ok(())
    }
}

impl Program {
    pub fn key(&self, root: &Path) -> Result<String, String> {
        let source = contained(&self.target.source, root)?;
        let source = source
            .strip_prefix(root)
            .map_err(|_| "test timing target leaves the workspace")?;
        let directory = contained(&self.directory, root)?;
        let directory = directory
            .strip_prefix(root)
            .map_err(|_| "test timing package leaves the workspace")?;
        let value = serde_json::json!({"directory":directory,"source":source,"name":self.target.name,
            "kind":self.target.kind,"features":self.features});
        Ok(plumb::depot::sha(
            &serde_json::to_vec(&value).map_err(|error| error.to_string())?,
        ))
    }
}
