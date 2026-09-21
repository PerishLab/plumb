use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Depot {
    pub source: String,
    pub derivatives: Vec<plumb::depot::v3::Kind>,
    #[serde(default)]
    pub validator: Vec<String>,
}

impl Depot {
    pub(super) fn validate(&self, binaries: &[String]) -> Result<(), String> {
        plumb::depot::v3::check(&self.source)?;
        if self.derivatives.is_empty() {
            return Err("depot must declare at least one derivative".into());
        }
        let mut derivatives = BTreeSet::new();
        for derivative in &self.derivatives {
            if !derivatives.insert(*derivative) {
                return Err(format!(
                    "depot repeats the {} derivative",
                    derivative.label()
                ));
            }
            if *derivative == plumb::depot::v3::Kind::Configuration && binaries.is_empty() {
                return Err(
                    "the configuration derivative requires an exact released binary".into(),
                );
            }
        }
        if derivatives.contains(&plumb::depot::v3::Kind::Configuration) {
            let Some(binary) = self.validator.first() else {
                return Err("the configuration derivative requires a validator command".into());
            };
            super::token("depot validator binary", binary, false)?;
            if !binaries.contains(binary) {
                return Err(format!("depot validator {binary} is not a released binary"));
            }
        } else if !self.validator.is_empty() {
            return Err("a knowledge-only depot cannot declare a validator command".into());
        }
        Ok(())
    }
}

impl super::Spec {
    pub fn route(&self, kind: plumb::depot::v3::Kind) -> Result<&str, String> {
        if !self.derivatives.contains(&kind) {
            return Err(format!(
                "release identity does not carry the {} derivative",
                kind.label()
            ));
        }
        self.route
            .as_deref()
            .ok_or_else(|| "release identity carries no depot authority".to_string())
    }
}
