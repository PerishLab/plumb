use super::{Closure, evidence::Source, registry::Entry};
use semver::{Version, VersionReq};

impl Closure<'_> {
    pub(super) fn expand(
        &mut self,
        reader: &mut impl Source,
        name: &str,
        requirement: &str,
    ) -> Result<(), String> {
        if self.failed.contains(name) {
            return Err(format!("declaration index remains unavailable: {name}"));
        }
        if !self.indices.contains_key(name) {
            let entries = reader
                .read(name)
                .and_then(|bytes| Entry::read(name, &bytes));
            match entries {
                Ok(entries) => {
                    self.indices.insert(name.into(), entries);
                }
                Err(error) => {
                    self.failed.insert(name.into());
                    return Err(error);
                }
            }
        }
        let requirement = VersionReq::parse(requirement).map_err(|error| error.to_string())?;
        let missing = self.locked.keys().any(|(package, version)| {
            package == name
                && Version::parse(version).is_ok_and(|v| requirement.matches(&v))
                && !self.indices[name]
                    .iter()
                    .any(|entry| &entry.vers == version)
        });
        if missing && self.refreshed.insert(name.into()) {
            match reader.refresh(name) {
                Ok(Some(bytes)) => {
                    self.indices.insert(name.into(), Entry::read(name, &bytes)?);
                }
                Ok(None) => {}
                Err(error) => {
                    self.failed.insert(name.into());
                    return Err(error);
                }
            }
        }
        let entries = self.indices[name]
            .iter()
            .filter(|entry| {
                Version::parse(&entry.vers).is_ok_and(|version| requirement.matches(&version))
            })
            .cloned()
            .collect::<Vec<_>>();
        if self.locked.keys().any(|(package, version)| {
            package == name
                && Version::parse(version).is_ok_and(|v| requirement.matches(&v))
                && !entries.iter().any(|entry| &entry.vers == version)
        }) {
            return Err(format!(
                "canonical index omits locked candidates for {name} {requirement}"
            ));
        }
        for entry in entries {
            entry.qualify(&self.locked)?;
            if !self.versions.insert((name.into(), entry.vers.clone())) {
                continue;
            }
            if self.versions.len() >= 50000 || self.edges >= 500000 {
                return Err("declaration candidate budget exhausted".into());
            }
            self.attach(&entry);
        }
        Ok(())
    }
}
