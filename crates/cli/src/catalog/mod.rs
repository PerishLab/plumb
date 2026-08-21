pub mod model;
pub mod query;
pub mod rules;
pub mod set;
pub mod taxonomy;

use model::{Coverage, Rule, Standing};
use std::collections::BTreeSet;
use std::sync::LazyLock;
use taxonomy::{Namespace, Owner, Tag};

pub fn all() -> Vec<&'static Rule> {
    let mut rules = rules::all();
    rules.sort_by(|one, two| one.id.cmp(&two.id));
    rules
}

pub(crate) struct Held {
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
    pub namespaces: Vec<Namespace>,
    pub rules: Vec<Rule>,
}

const WORDS: &str = "rules/taxonomy.toml";
const LAW: &str = "rules/catalog.toml";

pub(crate) fn held() -> &'static Held {
    &HELD
}

static HELD: LazyLock<Held> = LazyLock::new(|| {
    let words = plumb::seat::resource!("rules/taxonomy.toml");
    let law = plumb::seat::resource!("rules/catalog.toml");
    let seat = crate::command::depot::held();
    let carried = (
        seat.read(WORDS, words)
            .unwrap_or_else(|_| words.to_string()),
        seat.read(LAW, law).unwrap_or_else(|_| law.to_string()),
    );
    gather(&carried.0, &carried.1)
        .filter(covered)
        .unwrap_or_else(|| gather(words, law).expect("the compiled catalogue must hold"))
});

fn covered(held: &Held) -> bool {
    rules::mechanisms()
        .iter()
        .all(|mechanism| seek(&held.rules, mechanism.0).is_some())
}

pub(crate) fn seek<'a>(held: &'a [Rule], id: &str) -> Option<&'a Rule> {
    held.binary_search_by_key(&id, |rule| rule.id.as_str())
        .ok()
        .map(|seat| &held[seat])
}

fn gather(words: &str, law: &str) -> Option<Held> {
    let words: toml::Table = words.parse().ok()?;
    let owners = words
        .get("owner")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(Owner {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let tags = words
        .get("tag")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(Tag {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let namespaces = words
        .get("namespace")?
        .as_array()?
        .iter()
        .map(|value| {
            let owner = word(value, "owner")?;
            named(owners.iter().map(|owner| &owner.id), &owner)?;
            Some(Namespace {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
                owner,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let law: toml::Table = law.parse().ok()?;
    let mut rules = law
        .get("rule")?
        .as_array()?
        .iter()
        .map(|value| {
            let filed = value
                .get("tags")?
                .as_array()?
                .iter()
                .map(|tag| {
                    let tag = tag.as_str()?.to_string();
                    named(tags.iter().map(|held| &held.id), &tag)?;
                    Some(tag)
                })
                .collect::<Option<Vec<_>>>()?;
            let owner = word(value, "owner")?;
            named(owners.iter().map(|owner| &owner.id), &owner)?;
            Some(Rule {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
                law: word(value, "law")?,
                evidence: word(value, "evidence")?,
                standing: Standing::parse(&word(value, "standing")?)?,
                owner,
                tags: filed,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    rules.sort_by(|one, two| one.id.cmp(&two.id));
    Some(Held {
        owners,
        tags,
        namespaces,
        rules,
    })
}

fn word(value: &toml::Value, key: &str) -> Option<String> {
    Some(value.get(key)?.as_str()?.to_string())
}

fn named<'a>(mut held: impl Iterator<Item = &'a String>, id: &str) -> Option<()> {
    held.any(|held| held == id).then_some(())
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
    unique(
        "owner",
        taxonomy::owners().iter().map(|item| item.id.as_str()),
    )?;
    unique("tag", taxonomy::tags().iter().map(|item| item.id.as_str()))?;
    unique(
        "namespace",
        taxonomy::namespaces().iter().map(|item| item.id.as_str()),
    )?;
    unique("rule", all().iter().map(|rule| rule.id.as_str()))?;
    canonical(
        "owner",
        taxonomy::owners().iter().map(|item| item.id.as_str()),
    )?;
    canonical("tag", taxonomy::tags().iter().map(|item| item.id.as_str()))?;
    canonical(
        "namespace",
        taxonomy::namespaces().iter().map(|item| item.id.as_str()),
    )?;
    let owners = taxonomy::owners()
        .iter()
        .map(|owner| owner.id.as_str())
        .collect::<BTreeSet<_>>();
    for namespace in taxonomy::namespaces().iter() {
        if !owners.contains(namespace.owner.as_str()) {
            return Err(format!(
                "namespace {} names unknown owner {}",
                namespace.id, namespace.owner
            ));
        }
    }
    let namespaces = taxonomy::namespaces()
        .iter()
        .map(|namespace| namespace.id.as_str())
        .collect::<BTreeSet<_>>();
    let tags = taxonomy::tags()
        .iter()
        .map(|tag| tag.id.as_str())
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
        if !owners.contains(rule.owner.as_str()) {
            return Err(format!("rule {} names unknown owner", rule.id));
        }
        if rule.summary.is_empty() || rule.law.is_empty() || rule.evidence.is_empty() {
            return Err(format!("rule {} has an empty explanation", rule.id));
        }
        if rule.tags.is_empty() {
            return Err(format!("rule {} has no tags", rule.id));
        }
        for tag in &rule.tags {
            if !tags.contains(tag.as_str()) {
                return Err(format!("rule {} names unknown tag {}", rule.id, tag));
            }
        }
        unique(
            &format!("tag on rule {}", rule.id),
            rule.tags.iter().map(String::as_str),
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
