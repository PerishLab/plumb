use serde::{Deserialize, Serialize};
use serde_json::Value;

mod completion;
mod probe;
mod process;
mod production;
pub use completion::{Completion, Resource};
pub use probe::Probe;
pub use process::Observation;
pub use production::{Producer, Production, Receipt, Source};

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fields {
    pub pointer: String,
    pub allow: Option<Vec<String>>,
    pub deny: Vec<String>,
    pub required: Vec<String>,
}

impl Fields {
    pub fn check(&self, document: &Value) -> Result<Vec<String>, String> {
        self.validate()?;
        let Some(value) = document.pointer(&self.pointer) else {
            return Ok(vec![format!(
                "missing governed object at {:?}",
                self.pointer
            )]);
        };
        let Some(object) = value.as_object() else {
            return Ok(vec![format!(
                "governed value at {:?} is not an object",
                self.pointer
            )]);
        };
        let mut found = Vec::new();
        for required in &self.required {
            if !object.contains_key(required) {
                found.push(format!(
                    "missing required field {required:?} at {:?}",
                    self.pointer
                ));
            }
        }
        for name in object.keys() {
            if self.deny.contains(name)
                || self
                    .allow
                    .as_ref()
                    .is_some_and(|allow| !allow.contains(name))
            {
                found.push(format!("unapproved field {name:?} at {:?}", self.pointer));
            }
        }
        Ok(found)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.pointer.is_empty() && !self.pointer.starts_with('/') {
            return Err("field pointer must be empty or begin with /".into());
        }
        let mut chars = self.pointer.chars();
        while let Some(character) = chars.next() {
            if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
                return Err("field pointer carries an invalid escape".into());
            }
        }
        Ok(())
    }
}

pub fn document(path: &str, bytes: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "governed document is not UTF-8")?;
    match std::path::Path::new(path)
        .extension()
        .and_then(|held| held.to_str())
    {
        Some("json") => serde_json::from_str(text).map_err(|_| "invalid governed JSON".into()),
        Some("toml") => {
            let value: toml::Value = toml::from_str(text).map_err(|_| "invalid governed TOML")?;
            serde_json::to_value(value).map_err(|_| "cannot decode governed TOML".into())
        }
        _ => Err("structured field rules require a JSON or TOML document".into()),
    }
}
