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
    rules.sort_by_key(|rule| rule.id);
    rules
}

pub(crate) struct Held {
    pub owners: Vec<&'static Owner>,
    pub tags: Vec<&'static Tag>,
    pub namespaces: Vec<&'static Namespace>,
    pub rules: Vec<&'static Rule>,
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

pub(crate) fn seek(held: &[&'static Rule], id: &str) -> Option<&'static Rule> {
    held.binary_search_by_key(&id, |rule| rule.id)
        .ok()
        .map(|seat| held[seat])
}

fn gather(words: &str, law: &str) -> Option<Held> {
    let words: toml::Table = words.parse().ok()?;
    let owners = words
        .get("owner")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(&*Box::leak(Box::new(Owner {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
            })))
        })
        .collect::<Option<Vec<_>>>()?;
    let tags = words
        .get("tag")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(&*Box::leak(Box::new(Tag {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
            })))
        })
        .collect::<Option<Vec<_>>>()?;
    let namespaces = words
        .get("namespace")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(&*Box::leak(Box::new(Namespace {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
                owner: named(&owners, word(value, "owner")?)?,
            })))
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
                .map(|tag| named(&tags, Box::leak(tag.as_str()?.to_string().into_boxed_str())))
                .collect::<Option<Vec<_>>>()?;
            Some(&*Box::leak(Box::new(Rule {
                id: word(value, "id")?,
                summary: word(value, "summary")?,
                law: word(value, "law")?,
                evidence: word(value, "evidence")?,
                standing: Standing::parse(word(value, "standing")?)?,
                owner: named(&owners, word(value, "owner")?)?,
                tags: Box::leak(filed.into_boxed_slice()),
            })))
        })
        .collect::<Option<Vec<_>>>()?;
    rules.sort_by_key(|rule| rule.id);
    Some(Held {
        owners,
        tags,
        namespaces,
        rules,
    })
}

fn word(value: &toml::Value, key: &str) -> Option<&'static str> {
    let held = value.get(key)?.as_str()?;
    Some(Box::leak(held.to_string().into_boxed_str()))
}

fn named<T: Named>(held: &[&'static T], id: &str) -> Option<&'static T> {
    held.iter().copied().find(|item| item.id() == id)
}

pub(crate) trait Named {
    fn id(&self) -> &'static str;
}

impl Named for Owner {
    fn id(&self) -> &'static str {
        self.id
    }
}

impl Named for Tag {
    fn id(&self) -> &'static str {
        self.id
    }
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
    for namespace in taxonomy::NAMESPACES.iter() {
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
