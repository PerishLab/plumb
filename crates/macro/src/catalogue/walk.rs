use std::path::Path;

pub struct Held {
    pub files: Vec<(String, toml::Table)>,
}

const ROOTED: [&str; 2] = ["catalog.toml", "taxonomy.toml"];
const FOLDERS: [&str; 2] = ["atoms", "suites"];

pub fn read(root: &Path) -> Result<Held, String> {
    let mut files = Vec::new();
    for name in listed(root)? {
        let path = root.join(&name);
        if path.is_dir() {
            files.extend(inside(root, &name)?);
            continue;
        }
        if !ROOTED.contains(&name.as_str()) {
            return Err(format!(
                "rules/{name} is neither the catalogue, its taxonomy, an atom nor a suite"
            ));
        }
        files.push((name.clone(), parse(&path, &name)?));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Held { files })
}

fn inside(root: &Path, folder: &str) -> Result<Vec<(String, toml::Table)>, String> {
    if !FOLDERS.contains(&folder) {
        return Err(format!("rules/{folder} is neither atoms nor suites"));
    }
    let mut held = Vec::new();
    for name in listed(&root.join(folder))? {
        let inside = format!("{folder}/{name}");
        if !name.ends_with(".toml") {
            return Err(format!("rules/{inside} is not a rule source"));
        }
        held.push((
            inside.clone(),
            parse(&root.join(folder).join(&name), &inside)?,
        ));
    }
    Ok(held)
}

fn listed(root: &Path) -> Result<Vec<String>, String> {
    let mut held = std::fs::read_dir(root)
        .map_err(|error| format!("cannot read {}: {error}", root.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .map_err(|error| format!("cannot read {}: {error}", root.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    held.sort();
    Ok(held)
}

fn parse(path: &Path, name: &str) -> Result<toml::Table, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read rules/{name}: {error}"))?
        .parse()
        .map_err(|error| format!("rules/{name} does not parse: {error}"))
}
