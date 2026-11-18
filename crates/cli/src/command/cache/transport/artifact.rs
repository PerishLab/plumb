use super::inputs::digest;
use super::request::Client;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub(super) struct Listing {
    pub artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
pub(super) struct Artifact {
    pub id: u64,
    pub name: String,
    #[serde(rename = "size_in_bytes")]
    pub size: u64,
    pub digest: Option<String>,
    pub expired: bool,
    #[serde(rename = "workflow_run")]
    pub run: Option<Reference>,
}

#[derive(Deserialize)]
pub(super) struct Reference {
    pub id: u64,
}

pub(super) struct Slot {
    pub attempt: u64,
    pub exact: bool,
}

impl Artifact {
    pub fn slot(&self, family: &str, exact: &str) -> Option<Slot> {
        if self.expired || self.size == 0 || self.size > 10 * 1024 * 1024 * 1024 {
            return None;
        }
        self.digest
            .as_ref()?
            .strip_prefix("sha256:")
            .filter(|sha| super::context::hex(sha))?;
        let (producer, attempt) = self.name.strip_prefix(family)?.split_once('-')?;
        if !super::context::hex(producer) {
            return None;
        }
        Some(Slot {
            attempt: attempt
                .parse::<u64>()
                .ok()
                .filter(|attempt| *attempt != 0)?,
            exact: self.name.starts_with(exact),
        })
    }

    pub fn download(&self, client: &Client, repository: &str, target: &Path) -> Result<(), String> {
        crate::diagnostic::event(
            "material.candidate",
            || serde_json::json!({"artifact":self.id,"bytes":self.size,"run":self.run.as_ref().map(|run|run.id)}),
        );
        let parent = target.parent().ok_or("cache artifact has no parent")?;
        std::fs::create_dir_all(parent).map_err(|_| "cannot stage cache artifact")?;
        let archive =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "cannot stage cache artifact")?;
        crate::diagnostic::measure("material.download", || {
            client.download(
                &format!("repos/{repository}/actions/artifacts/{}/zip", self.id),
                archive.path(),
                self.size,
            )
        })?;
        let sha = format!(
            "sha256:{}",
            crate::diagnostic::measure("material.digest", || digest(archive.path()))?
        );
        if self.digest.as_deref() != Some(&sha) {
            return Err("GitHub cache artifact digest mismatch".into());
        }
        let mut zip =
            zip::ZipArchive::new(archive.reopen().map_err(|_| "cannot read cache artifact")?)
                .map_err(|_| "cache artifact is not a ZIP")?;
        if zip.len() != 1 {
            return Err("cache artifact contains undeclared files".into());
        }
        let mut entry = zip
            .by_index(0)
            .map_err(|_| "cannot read cache artifact entry")?;
        if Path::new(entry.name()) != target.file_name().ok_or("cache parcel has no name")? {
            return Err("cache artifact parcel name mismatch".into());
        }
        if entry.size() > 10 * 1024 * 1024 * 1024 {
            return Err("cache artifact parcel exceeds its bound".into());
        }
        if let Ok(metadata) = std::fs::symlink_metadata(target)
            && (!metadata.is_file() || metadata.file_type().is_symlink())
        {
            return Err("cache artifact refuses a foreign parcel".into());
        }
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "cannot stage cache parcel")?;
        std::io::copy(&mut entry, &mut file).map_err(|_| "cannot extract cache parcel")?;
        file.persist(target)
            .map_err(|_| "cannot publish cache parcel")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Artifact;

    fn artifact(name: String) -> Artifact {
        Artifact {
            id: 1,
            name,
            size: 1024,
            digest: Some(format!("sha256:{}", "a".repeat(64))),
            expired: false,
            run: None,
        }
    }

    fn name(world: char, producer: char, attempt: &str) -> String {
        format!(
            "guard-material-v1-{}-{}-{attempt}",
            world.to_string().repeat(64),
            producer.to_string().repeat(64)
        )
    }

    #[test]
    fn selection() {
        let family = format!("guard-material-v1-{}-", "a".repeat(64));
        let exact = format!("{family}{}-", "c".repeat(64));
        let mut held = artifact(name('a', 'c', "2"));
        let slot = held.slot(&family, &exact).unwrap();
        assert_eq!((slot.attempt, slot.exact), (2, true));
        held.name = name('a', 'd', "3");
        let slot = held.slot(&family, &exact).unwrap();
        assert_eq!((slot.attempt, slot.exact), (3, false));
        held.name = name('b', 'c', "2");
        assert!(held.slot(&family, &exact).is_none());
        held.name = name('a', 'c', "0");
        assert!(held.slot(&family, &exact).is_none());
        held.name = format!("{family}short-2");
        assert!(held.slot(&family, &exact).is_none());
        held.name = name('a', 'c', "2");
        held.expired = true;
        assert!(held.slot(&family, &exact).is_none());
        held.expired = false;
        held.digest = Some("sha256:unverified".into());
        assert!(held.slot(&family, &exact).is_none());
    }

    #[test]
    fn order() {
        let family = format!("guard-material-v1-{}-", "a".repeat(64));
        let exact = format!("{family}{}-", "c".repeat(64));
        let listed = [
            name('a', 'd', "9"),
            name('a', 'c', "1"),
            name('a', 'e', "5"),
        ];
        let mut found = listed
            .iter()
            .map(|name| artifact(name.clone()))
            .filter_map(|held| held.slot(&family, &exact).map(|slot| (held, slot)))
            .collect::<Vec<_>>();
        found.sort_by_key(|(_, slot)| !slot.exact);
        let names = found
            .iter()
            .map(|(held, _)| held.name.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [listed[1].clone(), listed[0].clone(), listed[2].clone()]
        );
    }
}
