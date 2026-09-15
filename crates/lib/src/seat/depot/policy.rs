use super::Rules;
use std::collections::BTreeMap;

pub fn policy(path: &str) -> bool {
    matches!(path.split('/').next(), Some("rules" | "profiles"))
}

impl Rules {
    pub fn inherit(
        &self,
        generation: &str,
        mut resources: BTreeMap<String, Vec<u8>>,
    ) -> Result<BTreeMap<String, Vec<u8>>, String> {
        if self.mark() != generation {
            return Err("configuration policy generation differs from its locked Profile".into());
        }
        for path in resources.keys() {
            super::anchored(path)?;
        }
        if let Some(path) = resources.keys().find(|path| policy(path)) {
            return Err(format!(
                "source resource cannot replace Depot policy {path}"
            ));
        }
        for object in self.objects().iter().filter(|object| policy(&object.path)) {
            resources.insert(object.path.clone(), self.read(&object.path)?.into_bytes());
        }
        Ok(resources)
    }
}
