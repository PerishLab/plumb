use serde_json::Value;

pub(super) fn judge(held: &Value, name: &str) -> Result<(), String> {
    if held["name"] != name || held["private"] != true {
        return Err("static app needs a matching private package".into());
    }
    let Some(scripts) = held["scripts"].as_object() else {
        return Err("static app needs an explicit build script".into());
    };
    let build = scripts.get("build").and_then(Value::as_str).unwrap_or("");
    if build.trim().is_empty() {
        return Err("static app needs a nonblank build script".into());
    }
    for key in ["prebuild", "postbuild"] {
        if scripts.contains_key(key) {
            return Err(format!("static app must not declare implicit {key} hooks"));
        }
    }
    Ok(())
}
