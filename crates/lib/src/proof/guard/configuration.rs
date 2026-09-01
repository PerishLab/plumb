use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const SCHEMA: &str = "plumb.guard-configuration/v1";
const LEAF: &str = "guard-configuration.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Validator {
    pub version: String,
    pub release: String,
    pub artifact: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    schema: String,
    target: String,
    validator: Validator,
    objects: Vec<crate::depot::Object>,
    digest: String,
}

#[derive(Serialize)]
struct Claim<'a> {
    schema: &'a str,
    target: &'a str,
    validator: &'a Validator,
    objects: &'a [crate::depot::Object],
}

impl Configuration {
    pub fn new(
        target: String,
        validator: Validator,
        mut objects: Vec<crate::depot::Object>,
    ) -> Result<Self, String> {
        objects.sort();
        let mut held = Self {
            schema: SCHEMA.into(),
            target,
            validator,
            objects,
            digest: String::new(),
        };
        held.digest = held.seal()?;
        held.validate()?;
        Ok(held)
    }

    pub fn install(&self, root: &Path, bodies: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
        self.validate()?;
        if bodies.len() != self.objects.len() {
            return Err("guard configuration bodies do not match its objects".into());
        }
        for object in &self.objects {
            let body = bodies
                .get(&object.path)
                .ok_or_else(|| format!("guard configuration carries no {}", object.path))?;
            self.verify(&object.path, body)?;
            write(&root.join(&object.path), body)?;
        }
        write(
            &root.join(LEAF),
            &serde_json::to_vec_pretty(self)
                .map_err(|error| format!("cannot encode guard configuration: {error}"))?,
        )
    }

    pub fn open(root: &Path, running: &str) -> Result<Self, String> {
        let path = root.join(LEAF);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let held: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot parse guard configuration: {error}"))?;
        held.validate()?;
        if held.target != running {
            return Err(format!(
                "guard configuration for {} cannot serve the running {running}",
                held.target
            ));
        }
        Ok(held)
    }

    pub fn read(&self, root: &Path, path: &str) -> Result<String, String> {
        anchored(path)?;
        let file = root.join(path);
        let bytes = std::fs::read(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        self.verify(path, &bytes)?;
        String::from_utf8(bytes).map_err(|error| format!("{path} is not UTF-8: {error}"))
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn objects(&self) -> &[crate::depot::Object] {
        &self.objects
    }

    fn verify(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let object = self
            .objects
            .iter()
            .find(|held| held.path == path)
            .ok_or_else(|| format!("guard configuration names no object at {path}"))?;
        if object.sha256 != crate::depot::sha(bytes) || object.size != bytes.len() as u64 {
            return Err(format!("guard configuration object drift: {path}"));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("guard configuration schema must be {SCHEMA}"));
        }
        version(&self.target, "guard configuration target")?;
        version(&self.validator.version, "guard configuration validator")?;
        digest(&self.validator.release, "validator release seal")?;
        digest(&self.validator.artifact, "validator artifact")?;
        digest(&self.digest, "guard configuration")?;
        if self.objects.is_empty() {
            return Err("guard configuration carries no objects".into());
        }
        let mut paths = BTreeSet::new();
        for object in &self.objects {
            anchored(&object.path)?;
            digest(&object.sha256, "guard configuration object")?;
            if !paths.insert(&object.path) {
                return Err(format!("guard configuration repeats {}", object.path));
            }
        }
        if !self.objects.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("guard configuration objects are not canonically ordered".into());
        }
        if self.seal()? != self.digest {
            return Err("guard configuration digest disagrees with its claim".into());
        }
        Ok(())
    }

    fn seal(&self) -> Result<String, String> {
        let claim = Claim {
            schema: &self.schema,
            target: &self.target,
            validator: &self.validator,
            objects: &self.objects,
        };
        serde_json::to_vec(&claim)
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
            .map_err(|error| format!("cannot seal guard configuration: {error}"))
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn anchored(raw: &str) -> Result<(), String> {
    if raw.is_empty()
        || raw.contains('\\')
        || Path::new(raw)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("guard configuration path is not anchored: {raw}"));
    }
    Ok(())
}

fn digest(raw: &str, name: &str) -> Result<(), String> {
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{name} is not a lowercase sha256 digest: {raw}"));
    }
    Ok(())
}

fn version(raw: &str, name: &str) -> Result<(), String> {
    semver::Version::parse(raw.trim_start_matches('v'))
        .map(|_| ())
        .map_err(|error| format!("cannot parse {name} {raw}: {error}"))
}
