use std::path::Path;

pub fn minted(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.contains("\"publishConfig\"") || text.contains("\"files\"")
}

pub fn field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let at = text.find(&marker)?;
    let rest = &text[at + marker.len()..];
    let colon = rest.find(':')?;
    let tail = &rest[colon + 1..];
    let open = tail.find('\"')?;
    let value = &tail[open + 1..];
    let close = value.find('\"')?;
    Some(value[..close].to_string())
}

pub fn name(dir: &Path) -> Option<String> {
    for seat in ["deno.json", "package.json"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(seat))
            && let Some(name) = field(&text, "name")
        {
            return Some(name);
        }
    }
    None
}
