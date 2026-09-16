use super::{Configuration, Target, configured};
use crate::shape::depot::Batch;
use std::collections::BTreeMap;
use std::path::Path;

pub fn review(root: &Path, bodies: &BTreeMap<String, Vec<u8>>) -> Result<Target, String> {
    configured(root, bodies)
}

impl Configuration for BTreeMap<String, Vec<u8>> {
    fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        match self.get(path) {
            Some(bytes) => String::from_utf8(bytes.clone())
                .map_err(|error| format!("candidate {path} is not UTF-8: {error}")),
            None => Ok(factory.to_string()),
        }
    }

    fn mark(&self) -> Option<String> {
        serde_json::to_vec(self)
            .ok()
            .map(|bytes| plumb::depot::sha(&bytes))
    }
}

pub fn resolve(root: &Path, batch: &Batch) -> Result<Target, String> {
    configured(root, batch)
}

impl Configuration for Batch {
    fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        let Some(bytes) = self.bodies.get(path) else {
            if self
                .manifest
                .objects
                .iter()
                .any(|object| object.path == path)
            {
                return Err(format!("candidate configuration is missing {path}"));
            }
            return Ok(factory.to_string());
        };
        self.manifest.verify(path, bytes)?;
        String::from_utf8(bytes.clone())
            .map_err(|error| format!("candidate {path} is not UTF-8: {error}"))
    }

    fn mark(&self) -> Option<String> {
        Some(self.manifest.snapshot.timestamp.clone())
    }
}
