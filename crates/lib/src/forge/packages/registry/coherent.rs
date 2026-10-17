use super::Entry;
use semver::VersionReq;
use std::collections::BTreeMap;

type Held<'a> = BTreeMap<&'a str, usize>;

pub(in super::super) fn select(
    indexes: &BTreeMap<String, Vec<Entry>>,
) -> Result<BTreeMap<String, String>, String> {
    let mut held = Held::new();
    for (name, entries) in indexes {
        if entries.is_empty() {
            return Err(format!("registry exposes no stable version of {name}"));
        }
        held.insert(name.as_str(), 0);
    }
    while let Some((name, dependency, req)) = conflict(indexes, &held) {
        step(indexes, &mut held, (name, dependency, req))?;
    }
    Ok(held
        .into_iter()
        .map(|(name, at)| (name.to_string(), indexes[name][at].version.to_string()))
        .collect())
}

fn conflict<'a>(
    indexes: &'a BTreeMap<String, Vec<Entry>>,
    held: &Held,
) -> Option<(&'a str, &'a str, &'a VersionReq)> {
    indexes.iter().find_map(|(name, entries)| {
        entries[held[name.as_str()]]
            .requires
            .iter()
            .find(|(dependency, req)| {
                held.get(dependency.as_str())
                    .is_some_and(|&at| !req.matches(&indexes[dependency][at].version))
            })
            .map(|(dependency, req)| (name.as_str(), dependency.as_str(), req))
    })
}

fn step<'a>(
    indexes: &'a BTreeMap<String, Vec<Entry>>,
    held: &mut Held<'a>,
    (name, dependency, req): (&'a str, &'a str, &VersionReq),
) -> Result<(), String> {
    let candidates = &indexes[dependency];
    let at = held[dependency];
    if let Some(back) =
        (at + 1..candidates.len()).find(|&index| req.matches(&candidates[index].version))
    {
        held.insert(dependency, back);
        return Ok(());
    }
    if held[name] + 1 < indexes[name].len() {
        held.insert(name, held[name] + 1);
        return Ok(());
    }
    Err(format!(
        "first-party Cargo crate {name} has no stable version whose {dependency} {req} the registry offers"
    ))
}

#[cfg(test)]
mod tests {
    use super::super as registry;

    fn line(version: &str, requires: &[(&str, &str)]) -> String {
        let deps = requires
            .iter()
            .map(|(name, req)| serde_json::json!({"name":name,"req":req,"kind":"normal","registry":null,"package":null}))
            .collect::<Vec<_>>();
        serde_json::json!({"name":"x","vers":version,"deps":deps,"yanked":false}).to_string()
    }

    fn pair(locus: &[String], locus_macro: &[String]) -> (String, String) {
        let indexes = [("locus", locus), ("locus-macro", locus_macro)]
            .into_iter()
            .map(|(name, lines)| {
                (
                    name.to_string(),
                    registry::entries(lines.join("\n").as_bytes()).unwrap(),
                )
            })
            .collect();
        let held = super::select(&indexes).unwrap();
        (held["locus"].clone(), held["locus-macro"].clone())
    }

    fn both(version: &str) -> (String, String) {
        (version.to_string(), version.to_string())
    }

    #[test]
    fn coherent() {
        let mut locus = vec![
            line("0.11.0", &[("locus-macro", "=0.11.0"), ("serde", "^1")]),
            line("0.12.0-rc.2", &[("locus-macro", "=0.12.0-rc.2")]),
        ];
        let early = [
            line("0.11.0", &[]),
            line("0.12.0-rc.2", &[]),
            line("0.12.0", &[]),
        ];
        assert_eq!(pair(&locus, &early), both("0.11.0"));
        locus.push(line("0.12.0", &[("locus-macro", "=0.12.0")]));
        assert_eq!(pair(&locus, &early), both("0.12.0"));
        assert_eq!(pair(&locus, &[line("0.11.0", &[])]), both("0.11.0"));
    }

    #[test]
    fn incoherent() {
        let indexes = [
            (
                "a".to_string(),
                registry::entries(line("1.0.0", &[("b", "=2.0.0")]).as_bytes()).unwrap(),
            ),
            (
                "b".to_string(),
                registry::entries(line("1.0.0", &[]).as_bytes()).unwrap(),
            ),
        ]
        .into_iter()
        .collect();
        assert!(
            super::select(&indexes)
                .unwrap_err()
                .contains("a has no stable version")
        );
    }
}
