use serde::Serialize;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Standing {
    #[serde(rename = "prose-only")]
    Prose,
    Observed,
    Mechanized,
    Retired,
}

impl Standing {
    pub fn id(self) -> &'static str {
        match self {
            Self::Prose => "prose-only",
            Self::Observed => "observed",
            Self::Mechanized => "mechanized",
            Self::Retired => "retired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "prose-only" => Some(Self::Prose),
            "observed" => Some(Self::Observed),
            "mechanized" => Some(Self::Mechanized),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

pub struct Rule {
    pub id: String,
    pub summary: String,
    pub law: String,
    pub evidence: String,
    pub standing: Standing,
    pub owner: String,
    pub tags: Vec<String>,
}

pub struct Mechanism(pub &'static str);

impl Deref for Mechanism {
    type Target = Rule;

    fn deref(&self) -> &Self::Target {
        super::rules::held(self.0)
    }
}

impl Rule {
    pub fn namespace(&self) -> &str {
        self.id
            .split_once('.')
            .map(|(namespace, _)| namespace)
            .expect("catalog validation requires a namespace")
    }

    pub fn name(&self) -> &str {
        self.id
            .split_once('.')
            .map(|(_, name)| name)
            .expect("catalog validation requires a name")
    }
}

#[derive(Serialize)]
pub struct View {
    pub id: &'static str,
    pub namespace: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub law: &'static str,
    pub evidence: &'static str,
    pub standing: Standing,
    pub owner: &'static str,
    pub tags: Vec<&'static str>,
}

impl From<&'static Rule> for View {
    fn from(rule: &'static Rule) -> Self {
        Self {
            id: &rule.id,
            namespace: rule.namespace(),
            name: rule.name(),
            summary: &rule.summary,
            law: &rule.law,
            evidence: &rule.evidence,
            standing: rule.standing,
            owner: &rule.owner,
            tags: rule.tags.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Coverage {
    #[serde(rename = "prose_only")]
    pub prose: usize,
    pub observed: usize,
    pub mechanized: usize,
    pub retired: usize,
}
