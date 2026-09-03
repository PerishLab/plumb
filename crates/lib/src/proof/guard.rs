use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

mod configuration;
mod store;
mod transit;

pub use configuration::{Configuration, Validator};
pub use transit::{attach, stage, staged};

pub const SCHEMA: &str = "plumb.guard-proof/v1";
pub const TRAILER: &str = "Plumb-Guard-Proof:";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub schema: String,
    pub repository: String,
    pub tree: String,
    pub plumb: String,
    pub depot: String,
    pub platform: String,
    pub actions: Vec<Action>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub name: String,
    pub input: String,
    pub world: String,
}

#[derive(Serialize)]
struct Claim<'a> {
    schema: &'a str,
    repository: &'a str,
    tree: &'a str,
    plumb: &'a str,
    depot: &'a str,
    platform: &'a str,
    actions: &'a [Action],
}

impl Descriptor {
    pub fn new(root: &Path, tree: String, actions: Vec<Action>) -> Result<Self, String> {
        if actions.is_empty() {
            return Err("guard proof names no action".into());
        }
        let mut held = Self {
            schema: SCHEMA.to_string(),
            repository: store::Seat::new(root)?.repository,
            tree,
            plumb: identity(),
            depot: crate::depot::rules()?.mark().to_string(),
            platform: crate::config::platform(),
            actions,
            digest: String::new(),
        };
        held.digest = held.seal()?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(|bytes| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
            .map_err(|error| format!("cannot encode guard proof: {error}"))
    }

    pub fn decode(text: &str) -> Result<Self, String> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(text.trim())
            .map_err(|error| format!("cannot decode guard proof: {error}"))?;
        let held: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot parse guard proof: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("guard proof schema must be {SCHEMA}"));
        }
        hash(&self.tree, "tree")?;
        hash(&self.digest, "digest")?;
        self.identities()?;
        for action in &self.actions {
            if action.name.trim().is_empty() {
                return Err("guard proof contains an unnamed action".into());
            }
            hash(&action.input, "action input")?;
            hash(&action.world, "action world")?;
        }
        if self.seal()? != self.digest {
            return Err("guard proof digest disagrees with its claim".into());
        }
        Ok(())
    }

    pub fn current(&self, root: &Path) -> Result<(), String> {
        self.subject(root)?;
        let depot = crate::depot::rules()?.mark();
        if self.depot != depot {
            return Err(format!(
                "guard proof used depot {}, not the active {depot}",
                self.depot
            ));
        }
        self.platform()
    }

    pub fn witness(&self, root: &Path) -> Result<(), String> {
        self.subject(root)?;
        self.platform()
    }

    fn subject(&self, root: &Path) -> Result<(), String> {
        store::Seat::new(root)?.matches(self)?;
        let plumb = identity();
        if self.plumb != plumb {
            return Err(format!(
                "guard proof used Plumb {}, not the running {plumb}",
                self.plumb
            ));
        }
        Ok(())
    }

    fn platform(&self) -> Result<(), String> {
        let platform = crate::config::platform();
        if self.platform != platform {
            return Err(format!(
                "guard proof used platform {}, not {platform}",
                self.platform
            ));
        }
        Ok(())
    }

    fn identities(&self) -> Result<(), String> {
        for (name, value) in [
            ("repository", &self.repository),
            ("Plumb", &self.plumb),
            ("depot", &self.depot),
            ("platform", &self.platform),
        ] {
            if value.trim().is_empty() {
                return Err(format!("guard proof contains an empty {name} identity"));
            }
        }
        if self.actions.is_empty() {
            return Err("guard proof contains no action identity".into());
        }
        Ok(())
    }

    fn seal(&self) -> Result<String, String> {
        let claim = Claim {
            schema: &self.schema,
            repository: &self.repository,
            tree: &self.tree,
            plumb: &self.plumb,
            depot: &self.depot,
            platform: &self.platform,
            actions: &self.actions,
        };
        serde_json::to_vec(&claim)
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
            .map_err(|error| format!("cannot seal guard proof: {error}"))
    }
}

pub fn current(root: &Path, commit: &str) -> Result<Descriptor, String> {
    let proof = self::commit(root, commit)?;
    proof.current(root)?;
    Ok(proof)
}

pub fn commit(root: &Path, commit: &str) -> Result<Descriptor, String> {
    store::Seat::new(root)?.committed(commit)
}

fn identity() -> String {
    match crate::commit!("PLUMB") {
        Some(commit) => format!("{}@{commit}", crate::version!("PLUMB")),
        None => crate::version!("PLUMB").to_string(),
    }
}

fn hash(value: &str, name: &str) -> Result<(), String> {
    let lowercase = value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if matches!(value.len(), 40 | 64) && lowercase {
        Ok(())
    } else {
        Err(format!(
            "guard proof {name} is not a lowercase object digest"
        ))
    }
}

fn home() -> Result<PathBuf, String> {
    crate::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::config::data("plumb"))
        .ok_or_else(|| "cannot stage a guard proof: no PLUMB_HOME".to_string())
}
