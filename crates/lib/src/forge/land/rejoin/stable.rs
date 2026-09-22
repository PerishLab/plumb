pub struct Stable {
    pub marker: String,
    pub commit: String,
}

pub fn latest<'a>(tags: impl IntoIterator<Item = (&'a str, &'a str)>) -> Option<Stable> {
    tags.into_iter()
        .filter_map(|(name, commit)| {
            let version = semver::Version::parse(name.strip_prefix('v')?).ok()?;
            version.pre.is_empty().then_some((version, name, commit))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, name, commit)| Stable {
            marker: name.to_string(),
            commit: commit.to_string(),
        })
}

pub fn tags(listing: &str) -> Vec<(&str, &str)> {
    let mut held = std::collections::BTreeMap::new();
    for line in listing.lines() {
        let Some((object, name)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = name.strip_prefix("refs/tags/") else {
            continue;
        };
        match name.strip_suffix("^{}") {
            Some(peeled) => {
                held.insert(peeled, object);
            }
            None => {
                held.entry(name).or_insert(object);
            }
        }
    }
    held.into_iter().collect()
}
