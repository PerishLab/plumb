pub mod model;
pub mod query;
pub mod rules;
pub mod set;
pub mod taxonomy;

use model::{Coverage, Rule, Standing};
use std::collections::BTreeSet;

pub fn all() -> Vec<&'static Rule> {
    let mut rules = rules::all();
    rules.sort_by_key(|rule| rule.id);
    rules
}

pub fn find(id: &str) -> Option<&'static Rule> {
    all().into_iter().find(|rule| rule.id == id)
}

pub fn coverage() -> Coverage {
    let mut coverage = Coverage {
        prose: 0,
        observed: 0,
        mechanized: 0,
    };
    for rule in all() {
        match rule.standing {
            Standing::Prose => coverage.prose += 1,
            Standing::Observed => coverage.observed += 1,
            Standing::Mechanized => coverage.mechanized += 1,
        }
    }
    coverage
}

pub fn validate() -> Result<(), String> {
    unique("owner", taxonomy::OWNERS.iter().map(|item| item.id))?;
    unique("tag", taxonomy::TAGS.iter().map(|item| item.id))?;
    unique("namespace", taxonomy::NAMESPACES.iter().map(|item| item.id))?;
    unique("rule", all().iter().map(|rule| rule.id))?;
    canonical("owner", taxonomy::OWNERS.iter().map(|item| item.id))?;
    canonical("tag", taxonomy::TAGS.iter().map(|item| item.id))?;
    canonical("namespace", taxonomy::NAMESPACES.iter().map(|item| item.id))?;
    let owners = taxonomy::OWNERS
        .iter()
        .map(|owner| owner.id)
        .collect::<BTreeSet<_>>();
    for namespace in taxonomy::NAMESPACES {
        if !owners.contains(namespace.owner.id) {
            return Err(format!(
                "namespace {} names unknown owner {}",
                namespace.id, namespace.owner.id
            ));
        }
    }
    let namespaces = taxonomy::NAMESPACES
        .iter()
        .map(|namespace| namespace.id)
        .collect::<BTreeSet<_>>();
    let tags = taxonomy::TAGS
        .iter()
        .map(|tag| tag.id)
        .collect::<BTreeSet<_>>();
    for rule in all() {
        let Some((namespace, name)) = rule.id.split_once('.') else {
            return Err(format!("rule {} has no namespace", rule.id));
        };
        if name.contains('.') || !token(namespace) || !token(name) {
            return Err(format!("rule {} is not a canonical id", rule.id));
        }
        if !namespaces.contains(namespace) {
            return Err(format!("rule {} names unknown namespace", rule.id));
        }
        if !owners.contains(rule.owner.id) {
            return Err(format!("rule {} names unknown owner", rule.id));
        }
        if rule.summary.is_empty() || rule.law.is_empty() || rule.evidence.is_empty() {
            return Err(format!("rule {} has an empty explanation", rule.id));
        }
        if rule.tags.is_empty() {
            return Err(format!("rule {} has no tags", rule.id));
        }
        for tag in rule.tags {
            if !tags.contains(tag.id) {
                return Err(format!("rule {} names unknown tag {}", rule.id, tag.id));
            }
        }
        unique(
            &format!("tag on rule {}", rule.id),
            rule.tags.iter().map(|tag| tag.id),
        )?;
    }
    Ok(())
}

fn unique<'a>(kind: &str, values: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(format!("duplicate {kind} {value}"));
        }
    }
    Ok(())
}

fn canonical<'a>(kind: &str, values: impl Iterator<Item = &'a str>) -> Result<(), String> {
    for value in values {
        if !token(value) {
            return Err(format!("{kind} {value} is not a canonical token"));
        }
    }
    Ok(())
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}
