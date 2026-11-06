use super::{Coverage, wire::Wire};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Normative,
    Diagnostic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub platform: String,
    pub backend: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub role: Role,
    pub subjects: Vec<String>,
    pub fixtures: Vec<String>,
    pub preparation: Vec<String>,
    pub conditions: Vec<Condition>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Subjects {
    pub schema: String,
    pub product: String,
    pub cases: Vec<Case>,
}

impl Subjects {
    pub const SCHEMA: &str = "plumb.release.subjects/v1";
    pub const PATH: &str = "subjects.json";

    pub fn parse(raw: &[u8]) -> Result<Self, String> {
        if raw.is_empty() || raw.len() > super::Request::LIMIT {
            return Err("subject declaration exceeds bounds".into());
        }
        let held: Self = serde_json::from_slice(raw)
            .map_err(|error| format!("invalid subject declaration: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != Self::SCHEMA || !(1..=1024).contains(&self.cases.len()) {
            return Err("unsupported or unbounded subject declaration".into());
        }
        Wire::name(&self.product)?;
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if !ids.insert(&case.id) {
                return Err("subject declaration repeats a case".into());
            }
        }
        Ok(())
    }

    pub fn bind(&self, cases: &[String], paths: &[String]) -> Result<(), String> {
        self.validate()?;
        let expected: BTreeSet<_> = cases.iter().collect();
        let declared: BTreeSet<_> = self.cases.iter().map(|case| &case.id).collect();
        if expected.len() != cases.len() || expected != declared {
            return Err("subject declaration must cover every indexed case exactly once".into());
        }
        for case in &self.cases {
            if case.evidence.iter().any(|path| !paths.contains(path)) {
                return Err(format!("case {} names absent evidence", case.id));
            }
        }
        Ok(())
    }

    pub fn coverage(&self, product: &str) -> Result<Vec<Coverage>, String> {
        self.validate()?;
        Wire::name(product)?;
        Ok(self
            .cases
            .iter()
            .filter(|case| {
                case.role == Role::Normative && case.subjects.iter().any(|name| name == product)
            })
            .flat_map(|case| {
                case.conditions.iter().map(|condition| Coverage {
                    case: case.id.clone(),
                    platform: condition.platform.clone(),
                    backend: condition.backend.clone(),
                })
            })
            .collect())
    }
}

impl Case {
    fn validate(&self) -> Result<(), String> {
        Wire::name(&self.id)?;
        Self::names(&self.subjects)?;
        Self::names(&self.fixtures)?;
        Self::names(&self.preparation)?;
        if self.subjects.is_empty() || !(1..=64).contains(&self.conditions.len()) {
            return Err("case requires explicit subjects and operating conditions".into());
        }
        if !(1..=128).contains(&self.evidence.len()) {
            return Err("case requires bounded explicit assertion evidence".into());
        }
        let mut evidence = BTreeSet::new();
        for path in &self.evidence {
            Wire::path(path)?;
            if !evidence.insert(path) {
                return Err("case repeats assertion evidence".into());
            }
        }
        let mut conditions = BTreeSet::new();
        for condition in &self.conditions {
            Wire::name(&condition.platform)?;
            Wire::name(&condition.backend)?;
            if !conditions.insert((&condition.platform, &condition.backend)) {
                return Err("case repeats operating conditions".into());
            }
        }
        Ok(())
    }

    fn names(names: &[String]) -> Result<(), String> {
        if names.len() > 64 || names.iter().collect::<BTreeSet<_>>().len() != names.len() {
            return Err("case has unbounded or duplicate product identities".into());
        }
        for name in names {
            Wire::name(name)?;
        }
        Ok(())
    }
}
