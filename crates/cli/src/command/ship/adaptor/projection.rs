use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

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

pub struct Workload<'a>(&'a [u8]);

impl<'a> Workload<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub fn bind(&self, output: &Path, package: &str, version: &Version) -> Result<(), String> {
        if self.current(package, version)? {
            return std::fs::write(output, self.0)
                .map_err(|error| format!("cannot create {}: {error}", output.display()));
        }
        let mut source = tar::Archive::new(GzDecoder::new(self.0));
        let file = std::fs::File::create(output)
            .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
        let encoder = GzBuilder::new()
            .mtime(0)
            .write(file, Compression::default());
        let mut target = tar::Builder::new(encoder);
        let mut manifest = false;
        for entry in source
            .entries()
            .map_err(|error| format!("cannot read reusable module workload: {error}"))?
        {
            let mut entry =
                entry.map_err(|error| format!("cannot read reusable module workload: {error}"))?;
            let path = entry
                .path()
                .map_err(|error| format!("cannot read reusable module path: {error}"))?
                .into_owned();
            let mut body = Vec::new();
            entry
                .read_to_end(&mut body)
                .map_err(|error| format!("cannot read reusable module entry: {error}"))?;
            let mut header = entry.header().clone();
            if path == Path::new("package/package.json") {
                let mut document: serde_json::Value = serde_json::from_slice(&body)
                    .map_err(|error| format!("cannot parse reusable package manifest: {error}"))?;
                if document.get("name").and_then(serde_json::Value::as_str) != Some(package) {
                    return Err(format!(
                        "reusable workload does not carry package {package}"
                    ));
                }
                document["version"] = serde_json::Value::String(version.to_string());
                body = serde_json::to_vec(&document)
                    .map_err(|error| format!("cannot encode reusable package manifest: {error}"))?;
                header.set_size(body.len() as u64);
                manifest = true;
            }
            header.set_mtime(0);
            header.set_cksum();
            target
                .append(&header, body.as_slice())
                .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
        }
        if !manifest {
            return Err("reusable workload carries no package/package.json".into());
        }
        target
            .into_inner()
            .and_then(flate2::write::GzEncoder::finish)
            .map_err(|error| format!("cannot finish {}: {error}", output.display()))?;
        Ok(())
    }

    fn current(&self, package: &str, version: &Version) -> Result<bool, String> {
        let mut source = tar::Archive::new(GzDecoder::new(self.0));
        let mut paths = std::collections::BTreeSet::new();
        let mut matched = None;
        for entry in source
            .entries()
            .map_err(|error| format!("cannot read reusable module workload: {error}"))?
        {
            let mut entry =
                entry.map_err(|error| format!("cannot read reusable module workload: {error}"))?;
            let path = entry
                .path()
                .map_err(|error| format!("cannot read reusable module path: {error}"))?
                .into_owned();
            if !path.starts_with("package")
                || path
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
                || !paths.insert(path.clone())
            {
                return Err("reusable module contains an unsafe or duplicate path".into());
            }
            if !entry.header().entry_type().is_file() && !entry.header().entry_type().is_dir() {
                return Err("reusable module requires ordinary files or directories".into());
            }
            if path != Path::new("package/package.json") {
                continue;
            }
            let mut body = Vec::new();
            entry
                .read_to_end(&mut body)
                .map_err(|error| format!("cannot read reusable module entry: {error}"))?;
            let document: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|error| format!("cannot parse reusable package manifest: {error}"))?;
            if document.get("name").and_then(serde_json::Value::as_str) != Some(package) {
                return Err(format!(
                    "reusable workload does not carry package {package}"
                ));
            }
            matched = Some(
                document.get("version").and_then(serde_json::Value::as_str)
                    == Some(version.to_string().as_str()),
            );
        }
        matched.ok_or_else(|| "reusable workload carries no package/package.json".into())
    }
}
