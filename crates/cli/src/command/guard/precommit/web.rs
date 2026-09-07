use super::super::workflow::tree::Tree;

pub(super) fn commands(tree: &Tree) -> Result<Vec<Vec<String>>, String> {
    let mut held = vec![
        vec!["corepack".into(), "enable".into()],
        vec!["pnpm".into(), "install".into(), "--frozen-lockfile".into()],
    ];
    if tree.has("biome.json") {
        held.push(vec!["pnpm".into(), "biome".into(), "ci".into(), ".".into()]);
    }
    held.push(vec![
        "pnpm".into(),
        "-r".into(),
        "exec".into(),
        "tsc".into(),
        "--noEmit".into(),
    ]);
    held.push(vec!["pnpm".into(), "-r".into(), "test".into()]);
    let mut sites = Vec::new();
    for path in tree.paths().filter(|path| manifest(path)) {
        let text = tree
            .text(path)?
            .ok_or_else(|| format!("cannot read staged manifest {path}"))?;
        let doc: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| format!("invalid staged package manifest {path}"))?;
        if doc.pointer("/scripts/build").is_some() {
            let name = doc
                .get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| format!("staged build manifest {path} requires a package name"))?;
            sites.push(name.to_string());
        }
    }
    sites.sort();
    for site in sites {
        held.push(vec!["pnpm".into(), "--filter".into(), site, "build".into()]);
    }
    Ok(held)
}

fn manifest(path: &str) -> bool {
    path.strip_prefix("apps/")
        .and_then(|path| path.strip_suffix("/package.json"))
        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}
