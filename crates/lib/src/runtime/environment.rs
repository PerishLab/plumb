use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::Command;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub inherit: Vec<String>,
    pub managed: Vec<String>,
    pub reject: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bind: BTreeMap<String, Binding>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Binding {
    Value(String),
    Tool { tool: String },
}

#[derive(Serialize)]
pub struct Environment {
    values: BTreeMap<String, String>,
    pub(crate) tools: BTreeMap<String, String>,
}

impl Binding {
    fn text(&self) -> &str {
        match self {
            Self::Value(value) => value,
            Self::Tool { tool } => tool,
        }
    }
}

impl Contract {
    pub fn capture(
        &self,
        values: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<Environment, String> {
        self.validate()?;
        let mut held = self
            .bind
            .iter()
            .map(|(key, value)| (key.clone(), value.text().to_string()))
            .collect::<BTreeMap<_, _>>();
        for (key, value) in values {
            let Some(key) = key.to_str() else { continue };
            if let Some(binding) = self.bind.get(key) {
                let value = value
                    .into_string()
                    .map_err(|_| format!("execution input {key} is not Unicode"))?;
                if matches!(binding, Binding::Value(expected) if expected != &value) {
                    return Err(format!(
                        "execution input {key} differs from its bound value"
                    ));
                }
                held.insert(key.to_string(), value);
                continue;
            }
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
        let tools = self
            .bind
            .iter()
            .filter_map(|(key, value)| match value {
                Binding::Tool { tool } => Some((key.clone(), tool.clone())),
                _ => None,
            })
            .collect();
        Ok(Environment {
            values: held,
            tools,
        })
    }

    fn validate(&self) -> Result<(), String> {
        for (key, binding) in &self.bind {
            if !token(key)
                || self.inherit.contains(key)
                || self.managed.iter().any(|pattern| matches(pattern, key))
            {
                return Err(format!("invalid bound execution input {key}"));
            }
            if let Binding::Tool { tool } = binding
                && (tool.is_empty()
                    || !tool
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte)))
            {
                return Err(format!(
                    "bound execution tool for {key} must be a program name"
                ));
            }
        }
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
    pub(crate) fn evidence(&self) -> BTreeMap<&str, &str> {
        self.values
            .iter()
            .filter(|(key, _)| key.as_str() != "PATH")
            .map(|(key, value)| (key.as_str(), self.tools.get(key).unwrap_or(value).as_str()))
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn apply(&self, command: &mut Command) {
        let overrides = command
            .get_envs()
            .map(|(key, value)| (key.to_os_string(), value.map(OsString::from)))
            .collect::<Vec<_>>();
        command.env_clear().envs(&self.values);
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
