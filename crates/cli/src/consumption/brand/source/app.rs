use super::super::Mode;
use super::{App, relative};
use crate::shape::web::Activation;
use std::path::Path;

pub(super) fn read(root: &Path, path: &Path, mode: Mode) -> Result<Option<App>, String> {
    let manifest = root.join(path).join("package.json");
    match std::fs::symlink_metadata(&manifest) {
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    let name = path.join("package.json");
    let bytes =
        super::super::material::regular(root, name.to_str().ok_or("non-text app manifest")?)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if !has(&manifest, "vite") || !has(&manifest, "svelte") {
        return Ok(None);
    }
    world(&root.join(path), &manifest)?;
    let configs = [
        "vite.config.ts",
        "vite.config.mts",
        "vite.config.js",
        "vite.config.mjs",
    ]
    .into_iter()
    .map(|name| root.join(path).join(name))
    .filter(|path| path.exists())
    .collect::<Vec<_>>();
    if configs.len() != 1 {
        return Err(format!(
            "{} needs one readable static Vite configuration",
            path.display()
        ));
    }
    let input = configs[0]
        .strip_prefix(root)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(super::super::material::regular(
        root,
        input.to_str().ok_or("non-text Vite config")?,
    )?)
    .map_err(|error| error.to_string())?;
    let setup = match Activation::read(&text) {
        Activation::Active(setup) => setup,
        Activation::Missing => {
            return Err(format!(
                "{} has no imported Design plugin activation",
                path.display()
            ));
        }
        Activation::Unknown(reason) => {
            return Err(format!(
                "{} Vite configuration is unqualified: {reason}",
                path.display()
            ));
        }
    };
    if let Some(problem) = &setup.problem {
        return Err(format!(
            "{} brand configuration is unqualified: {problem}",
            path.display()
        ));
    }
    if mode == Mode::Html && setup.options.get("brand").is_none() {
        return Err(format!(
            "{} has no explicit brand product/origin; adoption is unqualified",
            path.display()
        ));
    }
    relative(&setup.output)?;
    if !setup.base.starts_with('/') || setup.base.starts_with("//") || !setup.base.ends_with('/') {
        return Err("brand output requires a local absolute Vite base ending in /".into());
    }
    Ok(Some(App {
        path: path.into(),
        setup,
        mode,
        config: configs[0]
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("non-text Vite config")?
            .into(),
    }))
}

fn world(root: &Path, manifest: &serde_json::Value) -> Result<(), String> {
    if manifest.get("browser").is_some()
        || manifest
            .get("name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|name| ["@perishlab/design", "@perishlab/crest"].contains(&name))
    {
        return Err(
            "package browser/self-reference resolution is outside qualified brand scope".into(),
        );
    }
    for extension in ["js", "ts", "mjs", "mts", "cjs", "cts"] {
        let path = root.join(format!("svelte.config.{extension}"));
        match std::fs::symlink_metadata(path) {
            Ok(_) => {
                return Err(
                    "automatic Svelte configuration/transforms are outside qualified brand scope"
                        .into(),
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn has(manifest: &serde_json::Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|section| manifest[section][name].is_string())
}
