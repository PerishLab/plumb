use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::value::Value;
use super::{Bundle, LEAF, Manifest, Pointer};

const OBJECTS: &str = "objects";

#[derive(Clone, Debug)]
pub struct Cache {
    home: PathBuf,
}

struct Entry<'a>(&'a Path);

impl Cache {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    pub fn home() -> Result<Self, String> {
        let root = crate::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| crate::config::data("plumb"))
            .ok_or_else(|| "cannot locate depot cache: no PLUMB_HOME".to_string())?;
        Ok(Self::new(&root))
    }

    pub fn root(&self) -> PathBuf {
        self.home.join("depot").join("generations")
    }

    pub fn generation(&self, digest: &str) -> Result<PathBuf, String> {
        Value(digest).digest("depot generation")?;
        Ok(self.root().join(digest))
    }

    pub fn has(&self, digest: &str) -> Result<bool, String> {
        let entry = self.generation(digest)?;
        if !Entry(&self.home.join("depot")).present()? || !Entry(&self.root()).present()? {
            return Ok(false);
        }
        Entry(&entry).present()
    }

    pub fn read(&self, pointer: &Pointer) -> Result<Bundle, String> {
        let entry = self.generation(&pointer.generation)?;
        Entry(&self.home.join("depot")).directory()?;
        Entry(&self.root()).directory()?;
        Entry(&entry).directory()?;
        let raw = Entry(&entry.join(LEAF)).regular()?;
        let manifest = Manifest::parse(&raw)?;
        pointer.bind(&manifest, &raw)?;
        let objects = entry.join(OBJECTS);
        Entry(&objects).directory()?;
        let mut bodies = BTreeMap::new();
        for object in &manifest.objects {
            let bytes = Entry(&objects).nested(&object.path)?;
            manifest.verify(&object.path, &bytes, object.executable)?;
            bodies.insert(object.path.clone(), bytes);
        }
        Ok(Bundle { manifest, bodies })
    }

    pub fn store(&self, pointer: &Pointer, raw: &[u8], bundle: &Bundle) -> Result<PathBuf, String> {
        pointer.bind(&bundle.manifest, raw)?;
        verified(bundle)?;
        let generations = self.prepare()?;
        let target = generations.join(&pointer.generation);
        let stage = tempfile::Builder::new()
            .prefix(".stage-")
            .tempdir_in(&generations)
            .map_err(|error| {
                format!(
                    "cannot stage depot cache in {}: {error}",
                    generations.display()
                )
            })?;
        populate(stage.path(), raw, bundle)?;
        let staged = stage.keep();
        match fs::rename(&staged, &target) {
            Ok(()) => Ok(target),
            Err(error) => {
                let _ = fs::remove_dir_all(&staged);
                if matches!(error.kind(), ErrorKind::AlreadyExists)
                    || fs::symlink_metadata(&target).is_ok()
                {
                    self.read(pointer)?;
                    return Ok(target);
                }
                Err(format!(
                    "cannot commit depot cache {}: {error}",
                    target.display()
                ))
            }
        }
    }

    fn prepare(&self) -> Result<PathBuf, String> {
        fs::create_dir_all(&self.home)
            .map_err(|error| format!("cannot make {}: {error}", self.home.display()))?;
        let depot = self.home.join("depot");
        Entry(&depot).ensure()?;
        let generations = depot.join("generations");
        Entry(&generations).ensure()?;
        Ok(generations)
    }
}

impl Entry<'_> {
    fn present(&self) -> Result<bool, String> {
        match fs::symlink_metadata(self.0) {
            Ok(held) => {
                if held.file_type().is_symlink() || !held.file_type().is_dir() {
                    return Err(format!(
                        "depot cache path is not a plain directory: {}",
                        self.0.display()
                    ));
                }
                Ok(true)
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(format!(
                "cannot inspect depot cache {}: {error}",
                self.0.display()
            )),
        }
    }

    fn directory(&self) -> Result<(), String> {
        let held = fs::symlink_metadata(self.0)
            .map_err(|error| format!("cannot inspect depot cache {}: {error}", self.0.display()))?;
        if held.file_type().is_symlink() || !held.file_type().is_dir() {
            return Err(format!(
                "depot cache path is not a plain directory: {}",
                self.0.display()
            ));
        }
        Ok(())
    }

    fn ensure(&self) -> Result<(), String> {
        if !self.present()? {
            match fs::create_dir(self.0) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(format!(
                        "cannot make depot cache {}: {error}",
                        self.0.display()
                    ));
                }
            }
        }
        self.directory()
    }

    fn regular(&self) -> Result<Vec<u8>, String> {
        let held = fs::symlink_metadata(self.0)
            .map_err(|error| format!("cannot inspect depot cache {}: {error}", self.0.display()))?;
        if held.file_type().is_symlink() || !held.file_type().is_file() {
            return Err(format!(
                "depot cache object is not a regular file: {}",
                self.0.display()
            ));
        }
        fs::read(self.0)
            .map_err(|error| format!("cannot read depot cache {}: {error}", self.0.display()))
    }

    fn nested(&self, relative: &str) -> Result<Vec<u8>, String> {
        Value(relative).anchored("depot object path")?;
        let parts = relative.split('/').collect::<Vec<_>>();
        let mut current = self.0.to_path_buf();
        for part in &parts[..parts.len() - 1] {
            current.push(part);
            Entry(&current).directory()?;
        }
        current.push(parts[parts.len() - 1]);
        Entry(&current).regular()
    }

    fn write(&self, bytes: &[u8], executable: bool) -> Result<(), String> {
        let parent = self
            .0
            .parent()
            .ok_or_else(|| format!("{} has no parent", self.0.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
        fs::write(self.0, bytes)
            .map_err(|error| format!("cannot write {}: {error}", self.0.display()))?;
        self.mode(executable)
    }

    #[cfg(unix)]
    fn mode(&self, executable: bool) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt as _;
        let value = if executable { 0o755 } else { 0o644 };
        fs::set_permissions(self.0, fs::Permissions::from_mode(value))
            .map_err(|error| format!("cannot set mode on {}: {error}", self.0.display()))
    }

    #[cfg(not(unix))]
    fn mode(&self, _: bool) -> Result<(), String> {
        Ok(())
    }
}

fn verified(bundle: &Bundle) -> Result<(), String> {
    if bundle.bodies.len() != bundle.manifest.objects.len() {
        return Err("depot generation bodies do not match its manifest".into());
    }
    for object in &bundle.manifest.objects {
        let bytes = bundle
            .bodies
            .get(&object.path)
            .ok_or_else(|| format!("depot generation carries no body for {}", object.path))?;
        bundle
            .manifest
            .verify(&object.path, bytes, object.executable)?;
    }
    Ok(())
}

fn populate(stage: &Path, raw: &[u8], bundle: &Bundle) -> Result<(), String> {
    Entry(&stage.join(LEAF)).write(raw, false)?;
    let objects = stage.join(OBJECTS);
    fs::create_dir_all(&objects)
        .map_err(|error| format!("cannot make {}: {error}", objects.display()))?;
    for object in &bundle.manifest.objects {
        let bytes = &bundle.bodies[&object.path];
        let mut path = objects.clone();
        for part in object.path.split('/') {
            path.push(part);
        }
        Entry(&path).write(bytes, object.executable)?;
    }
    Ok(())
}
