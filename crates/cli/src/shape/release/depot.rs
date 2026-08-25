use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Depot {
    pub source: String,
    pub derivatives: Vec<plumb::depot::v2::Kind>,
    #[serde(default)]
    pub validator: Vec<String>,
}

impl Depot {
    pub(super) fn validate(&self, binaries: &[String]) -> Result<(), String> {
        if !self.source.starts_with("https://")
            || self.source.ends_with('/')
            || self.source.chars().any(char::is_whitespace)
        {
            return Err("depot source must be one normalized https URL".into());
        }
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
            if *derivative == plumb::depot::v2::Kind::Configuration && binaries.is_empty() {
                return Err(
                    "the configuration derivative requires an exact released binary".into(),
                );
            }
        }
        if derivatives.contains(&plumb::depot::v2::Kind::Configuration) {
            let Some(binary) = self.validator.first() else {
                return Err("the configuration derivative requires a validator command".into());
            };
            super::token("depot validator binary", binary, false)?;
            if !binaries.contains(binary) {
                return Err(format!("depot validator {binary} is not a released binary"));
            }
        } else if !self.validator.is_empty() {
            return Err("a changelog-only depot cannot declare a validator command".into());
        }
        Ok(())
    }
}

impl super::Spec {
    pub fn derivative(&self, kind: plumb::depot::v2::Kind) -> Result<&Depot, String> {
        let depot = self
            .depot
            .as_ref()
            .ok_or_else(|| "release declares no depot".to_string())?;
        if !depot.derivatives.contains(&kind) {
            return Err(format!(
                "release depot does not declare the {} derivative",
                kind.label()
            ));
        }
        Ok(depot)
    }
}
