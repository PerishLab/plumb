use std::collections::BTreeMap;
use std::path::Path;

const SEAT: &str = ".plumb/affirmed.toml";

pub(crate) struct Held(BTreeMap<String, String>);

impl Held {
    pub(crate) fn read(root: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(root.join(SEAT)) else {
            return Self(BTreeMap::new());
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            return Self(BTreeMap::new());
        };
        let mut held = BTreeMap::new();
        let listed = doc
            .get("record")
            .and_then(toml::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for entry in listed {
            let target = entry.get("target").and_then(toml::Value::as_str);
            let authority = entry.get("authority").and_then(toml::Value::as_str);
            if let (Some(target), Some(authority)) = (target, authority) {
                held.insert(target.to_string(), authority.to_string());
            }
        }
        Self(held)
    }

    pub(crate) fn authority(&self, target: &str) -> Option<&str> {
        self.0.get(target).map(String::as_str)
    }
}
