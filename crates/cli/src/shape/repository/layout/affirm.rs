use std::collections::BTreeMap;
use std::path::Path;

const SEAT: &str = ".plumb/affirmed.toml";

pub(crate) struct Held(BTreeMap<String, String>, Option<String>);

impl Held {
    pub(crate) fn read(root: &Path) -> Self {
        match crate::command::render::affirm::candidate::Review(root).selected() {
            Ok(Some(records)) => {
                return Self(
                    records
                        .into_iter()
                        .map(|record| (record.target, record.authority))
                        .collect(),
                    None,
                );
            }
            Err(error) => return Self(BTreeMap::new(), Some(error)),
            Ok(None) => (),
        }
        let Ok(text) = std::fs::read_to_string(root.join(SEAT)) else {
            return Self(BTreeMap::new(), None);
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            return Self(BTreeMap::new(), None);
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
        Self(held, None)
    }

    pub(crate) fn authority(&self, target: &str) -> Option<&str> {
        self.0.get(target).map(String::as_str)
    }

    pub(crate) fn refusal(&self) -> Option<&str> {
        self.1.as_deref()
    }
}
