use std::collections::BTreeMap;

use plumb::rule::Probe;
use serde::Deserialize;

use super::{Held, PLANES};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tool {
    program: String,
    platform: Option<Vec<String>>,
}

impl Tool {
    fn selected(&self, platform: &str) -> Result<Option<String>, String> {
        if self.program.is_empty()
            || !self
                .program
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        {
            return Err("workflow tool requires one program name".into());
        }
        if self
            .platform
            .as_ref()
            .is_some_and(|names| names.is_empty() || names.iter().any(|name| name.is_empty()))
        {
            return Err("workflow tool platform must name at least one platform".into());
        }
        Ok(self
            .platform
            .as_ref()
            .is_none_or(|names| names.iter().any(|name| name == platform))
            .then(|| self.program.clone()))
    }
}

impl Held {
    pub fn validate(&self) -> Result<(), String> {
        for action in self.probes.keys().chain(self.tools.keys()) {
            if !self.keys.iter().any(|key| key.name() == *action) {
                return Err(format!(
                    "workflow declaration names no action called {action}"
                ));
            }
        }
        Ok(())
    }

    pub fn probes(&self, action: &str) -> Result<Vec<Probe>, String> {
        let platform = plumb::config::platform();
        let mut selected = Vec::new();
        for probe in self.probes.get(action).into_iter().flatten() {
            probe.validate()?;
            if probe
                .platform
                .as_ref()
                .is_none_or(|names| names.iter().any(|name| name == &platform))
            {
                selected.push(probe.clone());
            }
        }
        Ok(selected)
    }

    pub fn tools(&self, action: &str) -> Result<Vec<String>, String> {
        let platform = plumb::config::platform();
        let mut selected = Vec::new();
        for tool in self.tools.get(action).into_iter().flatten() {
            if let Some(program) = tool.selected(&platform)? {
                selected.push(program);
            }
        }
        Ok(selected)
    }
}

pub(super) fn declarations<T: for<'de> Deserialize<'de>>(
    doc: &toml::Table,
    name: &str,
) -> Result<BTreeMap<String, Vec<T>>, String> {
    let Some(seat) = doc.get("workflow").and_then(|held| held.get(name)) else {
        return Ok(BTreeMap::new());
    };
    let Some(seat) = seat.as_table() else {
        return Err(format!("workflow.{name} must be a table"));
    };
    let mut found = BTreeMap::new();
    walk(seat, &mut Vec::new(), &mut found, name)?;
    Ok(found)
}

fn walk<T: for<'de> Deserialize<'de>>(
    seat: &toml::Table,
    segments: &mut Vec<String>,
    found: &mut BTreeMap<String, Vec<T>>,
    kind: &str,
) -> Result<(), String> {
    for (name, value) in seat {
        segments.push(name.clone());
        match value {
            toml::Value::Table(inner) => walk(inner, segments, found, kind)?,
            toml::Value::Array(_) => insert(value, segments, found, kind)?,
            _ => {
                return Err(format!(
                    "workflow.{kind}.{name} must hold a table or a list"
                ));
            }
        }
        segments.pop();
    }
    Ok(())
}

fn insert<T: for<'de> Deserialize<'de>>(
    value: &toml::Value,
    segments: &[String],
    found: &mut BTreeMap<String, Vec<T>>,
    kind: &str,
) -> Result<(), String> {
    let action = action(segments)?;
    let entries: Vec<T> = value
        .clone()
        .try_into()
        .map_err(|error| format!("workflow.{kind}.{action}: {error}"))?;
    if entries.is_empty() {
        return Err(format!("workflow.{kind}.{action} declares no entries"));
    }
    if found.insert(action.clone(), entries).is_some() {
        return Err(format!("workflow.{kind}.{action} is duplicated"));
    }
    Ok(())
}

fn action(segments: &[String]) -> Result<String, String> {
    match segments.split_first() {
        Some((lane, rest)) if PLANES.contains(&lane.as_str()) && !rest.is_empty() => {
            Ok(format!("{lane}/{}", rest.join(".")))
        }
        _ => Err("workflow declaration must name one guard or ship action".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse;

    const DECLARED: &str = r#"
[workflow.hash.guard]
rust = ["Cargo.toml"]

[[workflow.tool.guard.rust]]
program = "cmake"

[[workflow.probe.guard.rust]]
argv = ["perl", "-e", "print qq(ready)"]
stdout = "ready"
"#;

    #[test]
    fn accepts() {
        let held = parse(DECLARED);
        assert!(held.refusal.is_none());
        held.validate().unwrap();
        assert_eq!(held.tools("guard/rust").unwrap(), ["cmake"]);
        let probes = held.probes("guard/rust").unwrap();
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].argv[0], "perl");
    }

    #[test]
    fn action() {
        let held = parse(&DECLARED.replace("guard.rust", "guard.missing"));
        assert!(held.refusal.is_none());
        assert!(
            held.validate()
                .unwrap_err()
                .contains("no action called guard/missing")
        );
    }

    #[test]
    fn shape() {
        for source in [
            DECLARED.replace("program = \"cmake\"", "program = \"../cmake\""),
            DECLARED.replace("program = \"cmake\"", "program = \"cmake\"\nplatform = []"),
            DECLARED.replace("[[workflow.tool.guard.rust]]", "[workflow.tool.guard.rust]"),
        ] {
            let held = parse(&source);
            if held.refusal.is_none() {
                assert!(held.tools("guard/rust").is_err(), "{source}");
            }
        }
    }
}
