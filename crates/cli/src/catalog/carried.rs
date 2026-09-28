pub static FILES: &[(&str, &str)] = plumb::seat::catalogue!();

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
                let inside = inside
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                held.push(format!("rules/{inside}"));
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

    #[test]
    fn mark() {
        plumb::depot::carry(super::FILES);
        let manifest: toml::Value =
            toml::from_str(include_str!("../../../lib/Cargo.toml")).expect("library manifest");
        let declared = manifest["package"]["metadata"]["perish"]["guard"]["depot"]
            .as_str()
            .expect("released Depot mark");
        assert_eq!(
            plumb::depot::rules().expect("carried rules").mark(),
            declared
        );
    }
}
