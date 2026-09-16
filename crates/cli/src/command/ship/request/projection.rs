use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

pub(super) fn resolve(graph: &mut Value, root: &Path, commit: &str) -> Result<(), String> {
    let nodes = graph["nodes"].as_array_mut().ok_or("graph has no nodes")?;
    for node in nodes {
        let inputs = node["inputs"].as_object_mut().ok_or("node has no inputs")?;
        for input in inputs.values_mut() {
            if let Some(tree) = input.get_mut("tree") {
                prepare(tree, root, commit)?;
            }
        }
    }
    Ok(())
}

fn prepare(tree: &mut Value, root: &Path, commit: &str) -> Result<(), String> {
    let Some(projects) = tree.get("projects").and_then(Value::as_object) else {
        return Ok(());
    };
    let mut patches = serde_json::Map::new();
    for (path, recipe) in projects {
        if recipe["format"] != "toml" {
            continue;
        }
        if tree
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("product")
            != "product"
        {
            return Err(
                "projected control source requires its own resolved source authority".into(),
            );
        }
        let body = read(root, commit, path)?;
        patches.insert(path.clone(), patch(&body, recipe)?);
    }
    if !patches.is_empty() {
        tree["patches"] = Value::Object(patches);
    }
    Ok(())
}

fn read(root: &Path, commit: &str, path: &str) -> Result<String, String> {
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err("projection requires a relative source path".into());
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{commit}:{path}")])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("cannot read exact projected source {path}"));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

pub fn patch(body: &str, recipe: &Value) -> Result<Value, String> {
    let fields = recipe.as_object().ok_or("projection must be an object")?;
    if fields.len() != 2 || recipe["format"] != "toml" {
        return Err("unsupported TOML projection".into());
    }
    let sets = recipe["set"]
        .as_object()
        .ok_or("projection set must be a pointer map")?;
    let document = toml_edit::Document::parse(body).map_err(|error| error.to_string())?;
    let mut edits = Vec::new();
    for (pointer, value) in sets {
        let item = locate(document.as_item(), pointer)?;
        if item.as_value().is_none() {
            return Err("TOML projection must select a value, not a table declaration".into());
        }
        let span = item.span().ok_or("projected value has no source span")?;
        let replacement = toml::Value::try_from(value).map_err(|error| error.to_string())?;
        edits.push((span.start, span.end, replacement.to_string()));
    }
    edits.sort_by_key(|edit| edit.0);
    let mut result = String::new();
    let mut offset = 0;
    for (start, end, replacement) in &edits {
        if *start < offset {
            return Err("projection pointers overlap".into());
        }
        result.push_str(&body[offset..*start]);
        result.push_str(replacement);
        offset = *end;
    }
    result.push_str(&body[offset..]);
    let actual: toml::Value = toml::from_str(&result).map_err(|error| error.to_string())?;
    let mut expected: Value = serde_json::to_value(
        toml::from_str::<toml::Value>(body).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    for (pointer, value) in sets {
        *expected
            .pointer_mut(pointer)
            .ok_or("projection pointer is absent")? = value.clone();
    }
    if serde_json::to_value(actual).map_err(|error| error.to_string())? != expected {
        return Err("projected TOML changed undeclared values".into());
    }
    Ok(json!({"source": plumb::depot::sha(body.as_bytes()),
        "result": plumb::depot::sha(result.as_bytes()), "edits": edits}))
}

fn locate<'a>(mut item: &'a toml_edit::Item, pointer: &str) -> Result<&'a toml_edit::Item, String> {
    let pointer = pointer
        .strip_prefix('/')
        .ok_or("projection requires a JSON pointer")?;
    for token in pointer.split('/') {
        if token.replace("~0", "").replace("~1", "").contains('~') {
            return Err("invalid JSON pointer escape".into());
        }
        let token = token.replace("~1", "/").replace("~0", "~");
        item = if item.is_array() || item.is_array_of_tables() {
            let index: usize = token
                .parse()
                .map_err(|_| "invalid projection array index")?;
            if index.to_string() != token {
                return Err("invalid projection array index".into());
            }
            item.get(index)
        } else {
            item.get(&token)
        }
        .ok_or("projection pointer is absent")?;
    }
    Ok(item)
}
