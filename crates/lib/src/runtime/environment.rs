use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::Command;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub inherit: Vec<String>,
    pub managed: Vec<String>,
    pub reject: Vec<String>,
}

#[derive(Serialize)]
pub struct Environment(BTreeMap<String, String>);

impl Contract {
    pub fn capture(
        &self,
        values: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<Environment, String> {
        self.validate()?;
        let mut held = BTreeMap::new();
        for (key, value) in values {
            let Some(key) = key.to_str() else { continue };
            if self.managed.iter().any(|pattern| matches(pattern, key)) {
                continue;
            }
            if self.inherit.iter().any(|name| name == key) {
                let value = value
                    .into_string()
                    .map_err(|_| format!("execution input {key} is not Unicode"))?;
                held.insert(key.to_string(), value);
            } else if self.reject.iter().any(|pattern| matches(pattern, key)) {
                return Err(format!(
                    "execution contract refuses ambient {key}; use the Plumb-owned configuration"
                ));
            }
        }
        Ok(Environment(held))
    }

    fn validate(&self) -> Result<(), String> {
        for name in &self.inherit {
            if !token(name) || self.managed.iter().any(|pattern| matches(pattern, name)) {
                return Err(format!("invalid inherited execution input {name}"));
            }
        }
        for pattern in self.managed.iter().chain(&self.reject) {
            if !token(pattern.strip_suffix('*').unwrap_or(pattern)) {
                return Err(format!("invalid execution pattern {pattern}"));
            }
        }
        Ok(())
    }
}

impl Environment {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    pub fn apply(&self, command: &mut Command) {
        let overrides = command
            .get_envs()
            .map(|(key, value)| (key.to_os_string(), value.map(OsString::from)))
            .collect::<Vec<_>>();
        command.env_clear().envs(&self.0);
        for (key, value) in overrides {
            match value {
                Some(value) => {
                    command.env(key, value);
                }
                None => {
                    command.env_remove(key);
                }
            }
        }
    }
}

fn matches(pattern: &str, key: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => key.starts_with(prefix),
        None => key == pattern,
    }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
