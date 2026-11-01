use super::review::{Declaration, Mode};
use crate::catalog::rules::structure::LOCK_PROTECTION;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    rule: String,
    scope: Vec<Required>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Required {
    repository: String,
    features: Vec<Mode>,
}

pub(super) fn check(root: &Path, declaration: Option<&Declaration>) -> Result<(), String> {
    inspect(root, declaration).map_err(|error| format!("{}: {error}", LOCK_PROTECTION.id))
}

fn inspect(root: &Path, declaration: Option<&Declaration>) -> Result<(), String> {
    let rules = crate::catalog::set::read("structure")?;
    let policy: Policy = rules
        .get("lock")
        .ok_or("carried structure rules need a lock policy")?
        .clone()
        .try_into()
        .map_err(|error| format!("invalid carried lock policy: {error}"))?;
    policy.validate()?;
    if policy.scope.is_empty() {
        return Ok(());
    }
    let repository = plumb::guard::Repository::new(root)?.repository;
    let Some(required) = policy
        .scope
        .iter()
        .find(|scope| scope.repository.eq_ignore_ascii_case(&repository))
    else {
        return Ok(());
    };
    let declaration = declaration.ok_or_else(|| {
        format!("activated lock scope {repository} requires guard.lock.enforce=true")
    })?;
    let required = required.features.iter().collect::<BTreeSet<_>>();
    let declared = declaration.features.iter().collect::<BTreeSet<_>>();
    if required != declared {
        return Err(format!(
            "activated lock scope {repository} requires its exact carried feature coverage"
        ));
    }
    Ok(())
}

impl Policy {
    fn validate(&self) -> Result<(), String> {
        if self.rule != LOCK_PROTECTION.id {
            return Err("carried lock policy names a different rule".into());
        }
        let mut seen = BTreeSet::new();
        for scope in &self.scope {
            let parts = scope.repository.split('/').collect::<Vec<_>>();
            if parts.len() != 2
                || parts.iter().any(|part| {
                    part.is_empty()
                        || !part.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
                        })
                })
                || !seen.insert(scope.repository.to_ascii_lowercase())
            {
                return Err("carried lock scopes need unique owner/repository identities".into());
            }
            if scope.features.is_empty()
                || scope.features.iter().collect::<BTreeSet<_>>().len() != scope.features.len()
            {
                return Err("carried lock scopes need explicit unique feature coverage".into());
            }
        }
        Ok(())
    }
}
