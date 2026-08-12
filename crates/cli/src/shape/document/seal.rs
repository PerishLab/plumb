use super::config::{Binding, Strategy};
use plumb::snapshot::{Entry, Snapshot};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub struct Closure<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> Closure<'a> {
    pub fn new(snapshot: &'a Snapshot) -> Self {
        Self { snapshot }
    }

    pub fn source(&self, path: &str, documents: &BTreeSet<String>) -> Result<Metric, String> {
        let entries = self
            .seat(path)
            .into_iter()
            .filter(|entry| !document(entry.path(), documents))
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Err(format!(
                "source seat {path} has no tracked non-document leaf"
            ));
        }
        metric(entries, true)
    }

    pub fn target(&self, binding: &Binding) -> Result<Target, String> {
        let paths = binding.strategy.targets(binding.name.as_deref());
        let mut entries = Vec::new();
        for path in &paths {
            let Some(entry) = self
                .snapshot
                .entries()
                .iter()
                .find(|entry| entry.path() == path)
            else {
                return Err(format!("target {path} is not a tracked leaf"));
            };
            entries.push(entry);
        }
        let mut bytes = Vec::new();
        push(&mut bytes, binding.strategy.id().as_bytes());
        push(
            &mut bytes,
            binding.name.as_deref().unwrap_or_default().as_bytes(),
        );
        for source in &binding.sources {
            push(&mut bytes, source.path.as_bytes());
        }
        for entry in &entries {
            append(&mut bytes, entry, false)?;
        }
        let metric = metric(entries, false)?;
        Ok(Target {
            seal: digest(&bytes),
            lines: metric.lines,
            leaves: metric.leaves,
            paths,
        })
    }

    pub fn form(&self, binding: &Binding) -> Vec<String> {
        if binding.strategy != Strategy::Brief {
            return Vec::new();
        }
        let root = binding.strategy.target(binding.name.as_deref());
        let wanted = binding
            .strategy
            .targets(binding.name.as_deref())
            .into_iter()
            .collect::<BTreeSet<_>>();
        self.seat(&root)
            .into_iter()
            .filter(|entry| !wanted.contains(entry.path()))
            .map(|entry| entry.path().to_string())
            .collect()
    }

    fn seat(&self, path: &str) -> Vec<&'a Entry> {
        if path == "." {
            self.snapshot.entries().iter().collect()
        } else {
            self.snapshot.seat(path)
        }
    }
}

pub struct Metric {
    pub seal: String,
    pub lines: usize,
    pub leaves: usize,
}

pub struct Target {
    pub seal: String,
    pub lines: usize,
    pub leaves: usize,
    pub paths: Vec<String>,
}

fn metric(entries: Vec<&Entry>, semantic: bool) -> Result<Metric, String> {
    let mut bytes = Vec::new();
    let mut lines = 0;
    for entry in &entries {
        append(&mut bytes, entry, semantic)?;
        if let Ok(text) = std::str::from_utf8(entry.bytes()) {
            lines += count(text.as_bytes());
        }
    }
    Ok(Metric {
        seal: digest(&bytes),
        lines,
        leaves: entries.len(),
    })
}

fn append(bytes: &mut Vec<u8>, entry: &Entry, semantic: bool) -> Result<(), String> {
    push(bytes, entry.mode().as_bytes());
    push(bytes, entry.path().as_bytes());
    let body = if semantic && entry.path() == "plumb.toml" {
        manifest(entry.bytes())?
    } else {
        flat(entry.bytes())
    };
    push(bytes, &body);
    Ok(())
}

fn manifest(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "plumb.toml is not UTF-8".to_string())?;
    let mut table = text
        .parse::<toml::Table>()
        .map_err(|error| format!("cannot parse plumb.toml projection: {error}"))?;
    if let Some(list) = table
        .get_mut("document")
        .and_then(toml::Value::as_array_mut)
    {
        for document in list {
            scrub(document);
        }
    }
    toml::to_string(&table)
        .map(|text| text.into_bytes())
        .map_err(|error| format!("cannot render plumb.toml projection: {error}"))
}

fn scrub(value: &mut toml::Value) {
    let Some(document) = value.as_table_mut() else {
        return;
    };
    erase(document, "target-seal");
    let Some(sources) = document
        .get_mut("source")
        .and_then(toml::Value::as_array_mut)
    else {
        return;
    };
    for source in sources {
        if let Some(source) = source.as_table_mut() {
            erase(source, "seal");
        }
    }
}

fn erase(table: &mut toml::Table, key: &str) {
    if table.contains_key(key) {
        table.insert(key.into(), toml::Value::String(String::new()));
    }
}

fn document(path: &str, documents: &BTreeSet<String>) -> bool {
    documents.contains(path) || path == "docs/CHANGELOG" || path.starts_with("docs/CHANGELOG/")
}

fn flat(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut held = body.iter().peekable();
    while let Some(byte) = held.next() {
        if *byte == b'\r' && held.peek() == Some(&&b'\n') {
            continue;
        }
        out.push(*byte);
    }
    out
}

fn count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == b'\n').count()
        + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}

fn push(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(value.len().to_string().as_bytes());
    target.push(0);
    target.extend_from_slice(value);
}

fn digest(bytes: &[u8]) -> String {
    let mut sponge = Sha256::new();
    sponge.update(bytes);
    sponge
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
