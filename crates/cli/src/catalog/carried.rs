pub static FILES: &[(&str, &str)] = plumb::seat::catalogue!();

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    fn law() -> toml::Table {
        plumb::depot::carry(super::FILES);
        plumb::depot::rules()
            .expect("carried rules")
            .read(crate::catalog::LAW)
            .expect("carried catalogue")
            .parse()
            .expect("catalogue source")
    }

    #[test]
    fn listed() {
        let law = law();
        crate::catalog::prepare().expect("carried catalogue");
        let mut declared = law["rule"]
            .as_array()
            .expect("catalogue rules")
            .iter()
            .map(|rule| {
                let mut rule = serde_json::to_value(rule).expect("catalogue rule");
                let id = rule["id"].as_str().expect("rule identity").to_string();
                let (namespace, name) = id.split_once('.').expect("qualified identity");
                rule["namespace"] = namespace.into();
                rule["name"] = name.into();
                rule
            })
            .collect::<Vec<_>>();
        declared.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
        let listed = crate::catalog::all()
            .into_iter()
            .map(|rule| {
                serde_json::to_value(crate::catalog::model::View::from(rule)).expect("view")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            listed, declared,
            "the list shows the catalogue as it is carried"
        );
    }

    #[test]
    fn mechanized() {
        let law = law();
        let declared = law["rule"]
            .as_array()
            .expect("catalogue rules")
            .iter()
            .filter(|rule| rule["standing"].as_str() == Some("mechanized"))
            .map(|rule| rule["id"].as_str().expect("rule id").to_string())
            .collect::<BTreeSet<_>>();
        let implemented = crate::catalog::rules::mechanisms()
            .iter()
            .map(|mechanism| mechanism.0.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            declared.difference(&implemented).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "mechanized in the catalogue without a mechanism"
        );
        assert_eq!(
            implemented.difference(&declared).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "a mechanism whose rule the catalogue does not call mechanized"
        );
    }

    #[test]
    fn complete() {
        let carried = super::FILES
            .iter()
            .map(|(path, _)| *path)
            .collect::<Vec<_>>();
        let mut held = carried.clone();
        held.sort_unstable();
        held.dedup();
        assert_eq!(carried, held, "each rule source is carried once, in order");
        for rooted in ["rules/catalog.toml", "rules/taxonomy.toml"] {
            assert!(carried.contains(&rooted), "{rooted} is carried");
        }
        for folder in ["atoms", "suites"] {
            let prefix = format!("rules/{folder}/");
            assert!(
                carried.iter().any(|path| path.starts_with(&prefix)),
                "rules/{folder} carries at least one source"
            );
        }
        for path in &carried {
            let rooted = ["rules/catalog.toml", "rules/taxonomy.toml"].contains(path);
            let foldered = ["rules/atoms/", "rules/suites/"].iter().any(|prefix| {
                path.strip_prefix(prefix)
                    .is_some_and(|name| !name.contains('/') && name.ends_with(".toml"))
            });
            assert!(rooted || foldered, "{path} is not a rule source");
        }
        plumb::depot::carry(super::FILES);
        let loaded = plumb::depot::rules().expect("carried rules");
        for (path, body) in super::FILES {
            assert_eq!(loaded.read(path).expect("loaded rule"), *body, "{path}");
        }
    }

    #[test]
    fn mark() {
        plumb::depot::carry(super::FILES);
        assert_eq!(
            plumb::depot::rules().expect("carried rules").mark(),
            plumb::guard::Authority::pinned().expect("pinned Depot mark")
        );
    }
}
