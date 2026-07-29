use serde::Serialize;

pub type Found = Vec<Seed>;

#[derive(Clone)]
pub struct Seed {
    pub code: &'static str,
    pub grade: &'static str,
    pub evidence: String,
}

#[derive(Serialize)]
pub struct Finding {
    pub code: String,
    pub grade: &'static str,
    pub scope: &'static str,
    pub evidence: String,
    pub standing: &'static str,
}

impl Seed {
    pub fn wrong(code: &'static str, evidence: impl Into<String>) -> Self {
        Self {
            code,
            grade: "out of true",
            evidence: evidence.into(),
        }
    }

    pub fn blind(code: &'static str, evidence: impl Into<String>) -> Self {
        Self {
            code,
            grade: "blind",
            evidence: evidence.into(),
        }
    }

    pub fn unknown(code: &'static str, evidence: impl Into<String>) -> Self {
        Self {
            code,
            grade: "unknown shape",
            evidence: evidence.into(),
        }
    }
}

impl Finding {
    pub fn new(scope: &'static str, seed: Seed) -> Self {
        Self {
            code: format!("{scope}.{}", seed.code),
            grade: seed.grade,
            scope,
            evidence: seed.evidence,
            standing: "mechanized",
        }
    }
}
