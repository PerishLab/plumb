use semver::Version;
use std::ops::Range;

pub fn stamp(text: &str, version: &Version) -> Result<String, serde_json::Error> {
    let mut held: serde_json::Value = serde_json::from_str(text)?;
    if let Some(range) = property(text, "version") {
        let mut body = text.to_string();
        body.replace_range(range, &serde_json::to_string(&version.to_string())?);
        return Ok(body);
    }
    held["version"] = serde_json::Value::String(version.to_string());
    let mut body = serde_json::to_string_pretty(&held)?;
    body.push('\n');
    Ok(body)
}

fn property(text: &str, name: &str) -> Option<Range<usize>> {
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            let end = quoted(bytes, index)?;
            let found = field(text, name, depth, index..end);
            index = end;
            if found.is_some() {
                return found;
            }
            continue;
        }
        if matches!(bytes[index], b'{' | b'[') {
            depth += 1;
        } else if matches!(bytes[index], b'}' | b']') {
            depth -= 1;
        }
        index += 1;
    }
    None
}

fn field(text: &str, name: &str, depth: usize, key: Range<usize>) -> Option<Range<usize>> {
    if depth != 1 || serde_json::from_str::<String>(&text[key.clone()]).ok()? != name {
        return None;
    }
    let bytes = text.as_bytes();
    let colon = whitespace(bytes, key.end);
    if bytes.get(colon) != Some(&b':') {
        return None;
    }
    let value = whitespace(bytes, colon + 1);
    (bytes.get(value) == Some(&b'"')).then(|| value..quoted(bytes, value).unwrap_or(value))
}

fn whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn quoted(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}
