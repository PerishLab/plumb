use serde::Serialize;

use super::catalog::model::{MechanizedRule, Rule, Standing};

pub type Found = Vec<Seed>;

#[derive(Clone)]
pub struct Seed {
    pub rule: &'static Rule,
    pub grade: &'static str,
    pub evidence: String,
}

#[derive(Serialize)]
pub struct Finding {
    pub code: String,
    pub grade: &'static str,
    pub scope: &'static str,
    pub evidence: String,
    pub standing: Standing,
    pub owner: &'static str,
    pub tags: Vec<&'static str>,
}

impl Seed {
    pub fn wrong(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Self {
        Self {
            rule: &rule.0,
            grade: "out of true",
            evidence: evidence.into(),
        }
    }

    pub fn blind(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Self {
        Self {
            rule: &rule.0,
            grade: "blind",
            evidence: evidence.into(),
        }
    }

    pub fn unknown(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Self {
        Self {
            rule: &rule.0,
            grade: "unknown shape",
            evidence: evidence.into(),
        }
    }
}

impl Finding {
    pub fn new(seed: Seed) -> Self {
        Self {
            code: seed.rule.id.to_string(),
            grade: seed.grade,
            scope: seed.rule.namespace(),
            evidence: seed.evidence,
            standing: seed.rule.standing,
            owner: seed.rule.owner.id,
            tags: seed.rule.tags.iter().map(|tag| tag.id).collect(),
        }
    }
}

pub fn wrong(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Seed {
    Seed::wrong(rule, evidence)
}

pub fn blind(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Seed {
    Seed::blind(rule, evidence)
}

pub fn unknown(rule: &'static MechanizedRule, evidence: impl Into<String>) -> Seed {
    Seed::unknown(rule, evidence)
}
