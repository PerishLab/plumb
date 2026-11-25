use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub mod v3;

pub struct Rules {
    files: Vec<(String, Vec<u8>)>,
    mark: String,
    pinned: Option<v3::Pinned>,
}

static CARRIED: OnceLock<Rules> = OnceLock::new();

#[cfg(any(test, debug_assertions, feature = "testing"))]
pub fn carry(files: &'static [(&'static str, &'static str)]) {
    CARRIED.get_or_init(|| Rules::new(files));
}

pub fn install(rules: Rules) -> Result<(), String> {
    CARRIED
        .set(rules)
        .map_err(|_| "Plumb rules were already initialized".to_string())
}

pub fn rules() -> Result<&'static Rules, String> {
    CARRIED
        .get()
        .ok_or_else(|| "this binary carries no Plumb rules".to_string())
}

impl Rules {
    #[cfg(any(test, debug_assertions, feature = "testing"))]
    fn new(carried: &'static [(&'static str, &'static str)]) -> Self {
        let mut files = carried
            .iter()
            .map(|(path, body)| (path.to_string(), body.as_bytes().to_vec()))
            .collect::<Vec<_>>();
        for (path, body) in overlay() {
            match files.iter_mut().find(|(held, _)| *held == path) {
                Some(held) => held.1 = body.into_bytes(),
                None => files.push((path, body.into_bytes())),
            }
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        let objects = files
            .iter()
            .map(|(path, body)| v3::Object {
                path: path.clone(),
                sha256: sha(body),
                size: body.len() as u64,
                media: v3::media(path).to_string(),
                executable: false,
            })
            .collect::<Vec<_>>();
        let identity = v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: "v0.0.0".into(),
            marker: v3::Marker {
                name: "v0.0.0".into(),
                sha256: "0".repeat(64),
            },
            kind: v3::Kind::Configuration,
        };
        let manifest = v3::Manifest::new(identity, objects)
            .expect("carried Plumb rules form a valid Depot v3 manifest");
        let mark = manifest
            .generation()
            .expect("carried Plumb rules form a valid Depot v3 generation");
        Self {
            files,
            mark,
            pinned: None,
        }
    }

    pub fn bundle(bundle: &v3::Bundle) -> Result<Self, String> {
        if bundle.manifest.kind != v3::Kind::Configuration {
            return Err(format!(
                "depot rules require configuration kind, got {}",
                bundle.manifest.kind.label()
            ));
        }
        let mark = bundle.manifest.generation()?;
        if bundle.bodies.len() != bundle.manifest.objects.len() {
            return Err("depot configuration bodies do not match its manifest".into());
        }
        let mut files = Vec::with_capacity(bundle.manifest.objects.len());
        for object in &bundle.manifest.objects {
            let bytes = bundle.bodies.get(&object.path).ok_or_else(|| {
                format!("depot configuration carries no body for {}", object.path)
            })?;
            bundle
                .manifest
                .verify(&object.path, bytes, object.executable)?;
            files.push((object.path.clone(), bytes.clone()));
        }
        Ok(Self {
            files,
            mark,
            pinned: None,
        })
    }

    pub fn resolved(bundle: &v3::Bundle, pinned: v3::Pinned) -> Result<Self, String> {
        let mut rules = Self::bundle(bundle)?;
        if rules.mark != pinned.generation {
            return Err(format!(
                "depot configuration {} disagrees with pinned {}",
                rules.mark, pinned.generation
            ));
        }
        rules.pinned = Some(pinned);
        Ok(rules)
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let bytes = self
            .files
            .iter()
            .find(|(held, _)| held == path)
            .map(|(_, body)| body)
            .ok_or_else(|| format!("the carried Plumb rules hold no {path}"))?;
        String::from_utf8(bytes.clone())
            .map_err(|error| format!("the carried Plumb rule {path} is not UTF-8: {error}"))
    }

    pub fn mark(&self) -> &str {
        &self.mark
    }

    pub fn pinned(&self) -> Option<&v3::Pinned> {
        self.pinned.as_ref()
    }
}

pub fn sha(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(debug_assertions)]
fn overlay() -> Vec<(String, String)> {
    let Some(home) = crate::config::value("PLUMB_HOME") else {
        return Vec::new();
    };
    let root = std::path::Path::new(&home).join("overlay");
    let mut held = Vec::new();
    walk(&root, &root, &mut held);
    held
}

#[cfg(debug_assertions)]
fn walk(root: &std::path::Path, dir: &std::path::Path, held: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, held);
        } else if let (Ok(name), Ok(body)) =
            (path.strip_prefix(root), std::fs::read_to_string(&path))
        {
            held.push((name.to_string_lossy().replace('\\', "/"), body));
        }
    }
}

#[cfg(all(not(debug_assertions), any(test, feature = "testing")))]
fn overlay() -> Vec<(String, String)> {
    Vec::new()
}
