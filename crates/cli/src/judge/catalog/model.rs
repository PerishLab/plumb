use super::taxonomy::{Owner, Tag};
use serde::Serialize;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Standing {
    ProseOnly,
    Observed,
    Mechanized,
}

impl Standing {
    pub fn id(self) -> &'static str {
        match self {
            Self::ProseOnly => "prose-only",
            Self::Observed => "observed",
            Self::Mechanized => "mechanized",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "prose-only" => Some(Self::ProseOnly),
            "observed" => Some(Self::Observed),
            "mechanized" => Some(Self::Mechanized),
            _ => None,
        }
    }
}

pub struct Rule {
    pub id: &'static str,
    pub summary: &'static str,
    pub law: &'static str,
    pub evidence: &'static str,
    pub standing: Standing,
    pub owner: &'static Owner,
    pub tags: &'static [&'static Tag],
}

pub struct MechanizedRule(pub Rule);

impl Deref for MechanizedRule {
    type Target = Rule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Rule {
    pub fn namespace(&self) -> &'static str {
        self.id
            .split_once('.')
            .map(|(namespace, _)| namespace)
            .expect("catalog validation requires a namespace")
    }

    pub fn name(&self) -> &'static str {
        self.id
            .split_once('.')
            .map(|(_, name)| name)
            .expect("catalog validation requires a name")
    }
}

#[derive(Serialize)]
pub struct RuleView {
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

impl From<&'static Rule> for RuleView {
    fn from(rule: &'static Rule) -> Self {
        Self {
            id: rule.id,
            namespace: rule.namespace(),
            name: rule.name(),
            summary: rule.summary,
            law: rule.law,
            evidence: rule.evidence,
            standing: rule.standing,
            owner: rule.owner.id,
            tags: rule.tags.iter().map(|tag| tag.id).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Coverage {
    pub prose_only: usize,
    pub observed: usize,
    pub mechanized: usize,
}
