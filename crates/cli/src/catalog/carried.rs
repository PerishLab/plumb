pub static FILES: &[(&str, &str)] = &[
    (
        "rules/catalog.toml",
        plumb::seat::resource!("rules/catalog.toml"),
    ),
    ("rules/deps.toml", plumb::seat::resource!("rules/deps.toml")),
    (
        "rules/plumb.toml",
        plumb::seat::resource!("rules/plumb.toml"),
    ),
    (
        "rules/policy.toml",
        plumb::seat::resource!("rules/policy.toml"),
    ),
    (
        "rules/release.toml",
        plumb::seat::resource!("rules/release.toml"),
    ),
    ("rules/seat.toml", plumb::seat::resource!("rules/seat.toml")),
    (
        "rules/structure.toml",
        plumb::seat::resource!("rules/structure.toml"),
    ),
    (
        "rules/taxonomy.toml",
        plumb::seat::resource!("rules/taxonomy.toml"),
    ),
    (
        "rules/vocabulary.toml",
        plumb::seat::resource!("rules/vocabulary.toml"),
    ),
    (
        "rules/workflow.toml",
        plumb::seat::resource!("rules/workflow.toml"),
    ),
];

#[cfg(test)]
mod tests {
    #[test]
    fn complete() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");
        let mut held = std::fs::read_dir(&root)
            .expect("rules source")
            .map(|entry| {
                let name = entry.expect("rule entry").file_name();
                format!("rules/{}", name.to_string_lossy())
            })
            .collect::<Vec<_>>();
        held.sort();
        let carried = super::FILES
            .iter()
            .map(|(path, _)| path.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            carried, held,
            "every rule source is carried, and nothing else"
        );
    }
}
