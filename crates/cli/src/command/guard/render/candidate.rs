use super::receipt::Receipt;
use std::collections::BTreeMap;
use std::path::Path;

pub struct Review<'a>(pub &'a Path);

impl Review<'_> {
    pub fn run(&self, media: &Path, write: bool) -> i32 {
        match self.review(media, write) {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(error) => {
                eprintln!("plumb affirm: {error}");
                1
            }
        }
    }

    fn review(&self, media: &Path, write: bool) -> Result<String, String> {
        let bodies = plumb::depot::v3::Bundle::contents(media)?;
        let profile = crate::shape::product::review(self.0, &bodies)?
            .profile
            .ok_or("candidate review requires a Product Profile")?;
        let receipt = Receipt::read(self.0, &profile, &bodies)?;
        let bytes = receipt.encode()?;
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        if !write {
            return Ok(format!(
                "{text}\nRecord only after reading every document against this candidate configuration."
            ));
        }
        if receipt.owed().is_empty() {
            return Err("candidate profile asks for no document affirmation".into());
        }
        let path = media.join(receipt.path(&bytes));
        let parent = path.parent().ok_or("candidate affirmation has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut file, &bytes).map_err(|error| error.to_string())?;
        if path.exists() {
            if std::fs::read(&path).map_err(|error| error.to_string())? != bytes {
                return Err(format!(
                    "candidate affirmation already differs: {}",
                    path.display()
                ));
            }
        } else {
            file.persist_noclobber(&path)
                .map_err(|error| error.to_string())?;
        }
        Ok(format!("recorded reviewed documents in {}", path.display()))
    }

    pub fn project(&self, plan: &crate::shape::depot::Batch) -> Result<(), String> {
        if !present(plan.bodies.keys().map(String::as_str)) {
            return Ok(());
        }
        for (path, bytes) in &plan.bodies {
            plan.manifest.verify(path, bytes)?;
        }
        if let Some(owed) = self.confirmed(&plan.bodies)? {
            let root = self.0;
            let parent = root.join(".plumb");
            if parent.is_symlink() || root.join(super::SEAT).is_symlink() {
                return Err("candidate affirmation projection refuses symbolic links".into());
            }
            super::write(root, &owed)?;
        }
        Ok(())
    }

    pub fn selected(&self) -> Result<Option<Vec<super::Owed>>, String> {
        let crate::command::depot::Held::Seat(seat) = crate::command::depot::held() else {
            return Ok(None);
        };
        if !present(seat.objects().iter().map(|object| object.path.as_str())) {
            return Ok(None);
        }
        if crate::shape::product::governance(self.0)?.is_none() {
            return Ok(None);
        }
        let bodies = seat
            .objects()
            .iter()
            .filter(|object| plumb::depot::policy(&object.path))
            .map(|object| {
                seat.read(&object.path)
                    .map(|text| (object.path.clone(), text.into_bytes()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        self.confirmed(&bodies)
    }

    fn confirmed(
        &self,
        bodies: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Option<Vec<super::Owed>>, String> {
        let Some(profile) = crate::shape::product::review(self.0, bodies)?.profile else {
            return Ok(None);
        };
        if !bodies
            .keys()
            .any(|path| path.starts_with(&Receipt::namespace(&profile.digest)))
        {
            return Ok(None);
        }
        let receipt = Receipt::read(self.0, &profile, bodies)?;
        Ok(receipt.confirmed(bodies)?.then(|| receipt.owed()))
    }
}

fn present<'a>(paths: impl Iterator<Item = &'a str>) -> bool {
    paths
        .into_iter()
        .any(|path| path.starts_with("profiles/affirmed/"))
}
