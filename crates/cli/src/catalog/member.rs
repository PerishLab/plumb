pub(crate) struct Reference {
    pub set: String,
    pub slug: Option<String>,
}

pub(crate) struct Member {
    pub lines: Option<Lines>,
    pub fields: Vec<plumb::rule::Fields>,
    pub probe: Vec<plumb::rule::Probe>,
    pub allow: Option<Vec<String>>,
    pub deny: Vec<String>,
    pub holds: Option<String>,
    pub most: Option<usize>,
    pub leaf: Option<String>,
    pub bytes: Option<usize>,
    pub affirms: Vec<String>,
}

pub(crate) fn parse(held: &str) -> Result<Reference, String> {
    let rest = held
        .strip_prefix("rule://")
        .ok_or_else(|| format!("{held} carries no known scheme"))?;
    let (set, slug) = match rest.split_once('/') {
        Some((set, slug)) => (set, Some(slug.to_string())),
        None => (rest, None),
    };
    if set.is_empty() || slug.as_ref().is_some_and(String::is_empty) {
        return Err(format!("{held} names no rule"));
    }
    Ok(Reference {
        set: set.to_string(),
        slug,
    })
}

pub(crate) fn scan(held: &str) -> Result<toml::Table, String> {
    let reference = parse(held)?;
    let slug = reference
        .slug
        .as_deref()
        .ok_or_else(|| format!("{held} names a set and not one of its members"))?;
    crate::catalog::set::read(&reference.set)?
        .get("set")
        .and_then(toml::Value::as_array)
        .unwrap_or(&Vec::new())
        .iter()
        .find(|entry| entry.get("name").and_then(toml::Value::as_str) == Some(slug))
        .and_then(toml::Value::as_table)
        .cloned()
        .ok_or_else(|| format!("{held} names no set"))
}

pub(crate) fn member(reference: &Reference) -> Result<Member, String> {
    let doc = crate::catalog::set::read(&reference.set)?;
    let slug = reference
        .slug
        .as_deref()
        .ok_or_else(|| format!("rule://{} names a set and not a rule", reference.set))?;
    let listed = doc
        .get("member")
        .and_then(|value| value.get("entry"))
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let entry = listed
        .iter()
        .find(|entry| entry.get("name").and_then(toml::Value::as_str) == Some(slug))
        .ok_or_else(|| format!("rule://{}/{slug} names no rule", reference.set))?;
    Ok(Member {
        probe: entry
            .get("probe")
            .map(|value| {
                value
                    .clone()
                    .try_into()
                    .map_err(|error| format!("invalid member probe: {error}"))
            })
            .transpose()?
            .unwrap_or_default(),
        fields: entry
            .get("fields")
            .map(|value| {
                value
                    .clone()
                    .try_into()
                    .map_err(|error| format!("invalid member fields: {error}"))
            })
            .transpose()?
            .unwrap_or_default(),
        lines: entry
            .get("lines")
            .map(|value| {
                value
                    .clone()
                    .try_into()
                    .map_err(|error| format!("invalid member lines: {error}"))
            })
            .transpose()?,
        allow: names(entry, "allow")?,
        deny: names(entry, "deny")?.unwrap_or_default(),
        holds: entry
            .get("holds")
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        most: sized(entry, "most"),
        leaf: entry
            .get("leaf")
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        bytes: sized(entry, "bytes"),
        affirms: entry
            .get("affirms")
            .and_then(toml::Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn sized(entry: &toml::Value, key: &str) -> Option<usize> {
    entry
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|held| usize::try_from(held).ok())
}

fn names(entry: &toml::Value, key: &str) -> Result<Option<Vec<String>>, String> {
    let Some(value) = entry.get(key) else {
        return Ok(None);
    };
    let list = value
        .as_array()
        .ok_or_else(|| format!("member {key} must be a name list"))?;
    list.iter()
        .map(|value| {
            let name = value
                .as_str()
                .ok_or_else(|| format!("member {key} must hold names"))?;
            if name.is_empty() || name.contains(['/', '\\', '*']) || matches!(name, "." | "..") {
                return Err(format!("member {key} contains an invalid name {name}"));
            }
            Ok(name.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub fn named(name: &str, holds: &str, repository: &str) -> bool {
    match holds {
        "repository" => name == repository,
        "version" => versioned(name),
        _ => true,
    }
}

#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Lines {
    pub allow: Option<Vec<String>>,
    pub deny: Vec<String>,
    pub required: Vec<String>,
}

pub fn known(holds: &str) -> bool {
    matches!(holds, "repository" | "version")
}

pub fn face(name: &str) -> bool {
    matches!(name, "declaration" | "seat" | "lane")
}

fn versioned(name: &str) -> bool {
    let rest = name.strip_prefix('v').unwrap_or(name);
    let mut parts = rest.split('.');
    let held = [parts.next(), parts.next(), parts.next()];
    if parts.next().is_some() {
        return false;
    }
    held.iter().all(|part| {
        part.is_some_and(|held| {
            !held.is_empty() && held.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
    })
}
