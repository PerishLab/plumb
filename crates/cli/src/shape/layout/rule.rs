pub struct Reference {
    pub set: String,
    pub slug: Option<String>,
}

pub struct Member {
    pub holds: Option<String>,
    pub count: Option<usize>,
    pub leaf: Option<String>,
    pub bytes: Option<usize>,
    pub affirms: Vec<String>,
}

pub fn parse(held: &str) -> Result<Reference, String> {
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

pub fn member(reference: &Reference) -> Result<Member, String> {
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
        holds: entry
            .get("holds")
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        count: sized(entry, "count"),
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

pub fn named(name: &str, holds: &str, repository: &str) -> bool {
    match holds {
        "repository" => name == repository,
        "version" => versioned(name),
        _ => true,
    }
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
