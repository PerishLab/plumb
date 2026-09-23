pub static FILES: &[(&str, &str)] = &[
    (
        "rules/catalog.toml",
        plumb::seat::resource!("rules/catalog.toml"),
    ),
    (
        "rules/taxonomy.toml",
        plumb::seat::resource!("rules/taxonomy.toml"),
    ),
    (
        "rules/atoms/deps.toml",
        plumb::seat::resource!("rules/atoms/deps.toml"),
    ),
    (
        "rules/atoms/inputs.toml",
        plumb::seat::resource!("rules/atoms/inputs.toml"),
    ),
    (
        "rules/atoms/limit.toml",
        plumb::seat::resource!("rules/atoms/limit.toml"),
    ),
    (
        "rules/atoms/release.toml",
        plumb::seat::resource!("rules/atoms/release.toml"),
    ),
    (
        "rules/atoms/scan.toml",
        plumb::seat::resource!("rules/atoms/scan.toml"),
    ),
    (
        "rules/atoms/seat.toml",
        plumb::seat::resource!("rules/atoms/seat.toml"),
    ),
    (
        "rules/atoms/structure.toml",
        plumb::seat::resource!("rules/atoms/structure.toml"),
    ),
    (
        "rules/atoms/vocabulary.toml",
        plumb::seat::resource!("rules/atoms/vocabulary.toml"),
    ),
    (
        "rules/suites/layout.toml",
        plumb::seat::resource!("rules/suites/layout.toml"),
    ),
    (
        "rules/suites/shape.toml",
        plumb::seat::resource!("rules/suites/shape.toml"),
    ),
];

#[cfg(test)]
mod tests {
    #[test]
    fn complete() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");
        let mut held = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).expect("rules source") {
                let path = entry.expect("rule entry").path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let inside = path.strip_prefix(&root).expect("rule path");
                held.push(format!("rules/{}", inside.to_string_lossy()));
            }
        }
        held.sort();
        let mut carried = super::FILES
            .iter()
            .map(|(path, _)| path.to_string())
            .collect::<Vec<_>>();
        carried.sort();
        assert_eq!(
            carried, held,
            "every rule source is carried, and nothing else"
        );
    }
}
