use std::collections::BTreeSet;
use std::path::Path;
use toml_edit::{DocumentMut, Item};

pub(super) struct Edit {
    path: String,
    before: String,
    after: String,
}

type Names = BTreeSet<(String, String)>;

pub(super) fn read(root: &Path, path: &str, names: &mut Names) -> Result<Option<Edit>, String> {
    let file = path.rsplit('/').next().unwrap_or_default();
    if !matches!(file, "Cargo.toml" | "package.json") {
        return Ok(None);
    }
    let seat = root.join(path);
    if std::fs::symlink_metadata(&seat)
        .map_err(|error| error.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err(format!("dependency manifest is a symlink: {path}"));
    }
    let before =
        std::fs::read_to_string(seat).map_err(|error| format!("cannot read {path}: {error}"))?;
    let after = if file == "Cargo.toml" {
        cargo(&before, names)?
    } else {
        npm(&before, names)?
    };
    Ok((before != after).then(|| Edit {
        path: path.into(),
        before,
        after,
    }))
}

impl Edit {
    pub(super) fn npm(&self) -> bool {
        self.path.ends_with("package.json")
    }

    pub(super) fn write(&self, root: &Path) -> Result<(), String> {
        let path = root.join(&self.path);
        let current = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        if current != self.before {
            return Err(format!("manifest changed after resolution: {}", self.path));
        }
        std::fs::write(path, &self.after).map_err(|error| error.to_string())
    }
}

fn cargo(text: &str, names: &mut Names) -> Result<String, String> {
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    walk(document.as_item_mut(), names)?;
    Ok(document.to_string())
}

fn walk(item: &mut Item, names: &mut Names) -> Result<(), String> {
    let Some(table) = item.as_table_like_mut() else {
        return Ok(());
    };
    for (key, value) in table.iter_mut() {
        match key.get() {
            "dependencies" | "dev-dependencies" | "build-dependencies" => {
                dependencies(value, names)?
            }
            "workspace" => walk(value, names)?,
            "target" => targets(value, names)?,
            _ => {}
        }
    }
    Ok(())
}

fn targets(item: &mut Item, names: &mut Names) -> Result<(), String> {
    let Some(table) = item.as_table_like_mut() else {
        return Ok(());
    };
    for (_, value) in table.iter_mut() {
        walk(value, names)?;
    }
    Ok(())
}

fn dependencies(item: &mut Item, names: &mut Names) -> Result<(), String> {
    let Some(table) = item.as_table_like_mut() else {
        return Ok(());
    };
    for (key, item) in table.iter_mut() {
        let Some(spec) = item.as_table_like_mut() else {
            continue;
        };
        if spec.get("registry").and_then(Item::as_str) != Some("perish")
            || spec.contains_key("path")
        {
            continue;
        }
        let name = spec
            .get("package")
            .and_then(Item::as_str)
            .unwrap_or(key.get())
            .to_string();
        names.insert(("cargo".into(), name));
        if spec.get("version").and_then(Item::as_str) != Some("0") {
            spec.insert("version", toml_edit::value("0"));
        }
    }
    Ok(())
}

fn npm(text: &str, names: &mut Names) -> Result<String, String> {
    let mut document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut changed = false;
    for kind in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        let Some(table) = document
            .get_mut(kind)
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        for (name, requirement) in table {
            if !name.starts_with("@perishlab/")
                || requirement.as_str().is_some_and(|value| {
                    value.starts_with("workspace:")
                        || value.starts_with("file:")
                        || value.starts_with("link:")
                })
            {
                continue;
            }
            names.insert(("npm".into(), name.clone()));
            if requirement.as_str() != Some("0") {
                *requirement = "0".into();
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(text.into());
    }
    serde_json::to_string_pretty(&document)
        .map(|text| format!("{text}\n"))
        .map_err(|error| error.to_string())
}
