use serde::Serialize;

use crate::catalog::model::{Mechanism, Rule, Standing};

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
    pub fn wrong(rule: &'static Mechanism, evidence: impl Into<String>) -> Self {
        Self {
            rule: &**rule,
            grade: "out of true",
            evidence: evidence.into(),
        }
    }

    pub fn blind(rule: &'static Mechanism, evidence: impl Into<String>) -> Self {
        Self {
            rule: &**rule,
            grade: "blind",
            evidence: evidence.into(),
        }
    }

    pub fn noted(rule: &'static Mechanism, evidence: impl Into<String>) -> Self {
        Self {
            rule: &**rule,
            grade: "noted",
            evidence: evidence.into(),
        }
    }

    pub fn unknown(rule: &'static Mechanism, evidence: impl Into<String>) -> Self {
        Self {
            rule: &**rule,
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
            owner: &seed.rule.owner,
            tags: seed.rule.tags.iter().map(String::as_str).collect(),
        }
    }
}

pub fn wrong(rule: &'static Mechanism, evidence: impl Into<String>) -> Seed {
    Seed::wrong(rule, evidence)
}

pub fn blind(rule: &'static Mechanism, evidence: impl Into<String>) -> Seed {
    Seed::blind(rule, evidence)
}

pub fn unknown(rule: &'static Mechanism, evidence: impl Into<String>) -> Seed {
    Seed::unknown(rule, evidence)
}
