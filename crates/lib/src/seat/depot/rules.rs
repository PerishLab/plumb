use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{Object, Seat, anchored, v2, v3};
pub mod exact;
pub use exact::Selection;

pub struct Rules {
    held: Source,
}

enum Source {
    Guard {
        base: PathBuf,
        manifest: crate::guard::Configuration,
    },
    V1(Seat),
    V2 {
        base: PathBuf,
        manifest: v2::Manifest,
    },
    V3 {
        base: PathBuf,
        manifest: v3::Manifest,
        generation: String,
        objects: Vec<Object>,
    },
    #[cfg(feature = "depot")]
    Remote(v3::Exact, Vec<Object>),
}

enum Width {
    Exact,
    Floor,
}

static RULES: LazyLock<Result<Rules, String>> = LazyLock::new(Rules::open);

pub(super) fn held() -> Result<&'static Rules, String> {
    RULES.as_ref().map_err(Clone::clone)
}

impl Rules {
    fn open() -> Result<Self, String> {
        if crate::config::value("PLUMB_HOME").is_none()
            && let Some(root) = crate::config::value("PLUMB_GUARD_CONFIGURATION")
        {
            return Self::guard(Path::new(&root), crate::version!("PLUMB"));
        }
        if let Some(root) = crate::config::value("PLUMB_DEPOT_SNAPSHOT") {
            return Self::staged(Path::new(&root), crate::version!("PLUMB"));
        }
        Self::at(&super::root(&PathBuf::new())?, crate::version!("PLUMB"))
    }

    pub fn guard(base: &Path, running: &str) -> Result<Self, String> {
        let manifest = crate::guard::Configuration::open(base, running)?;
        Ok(Self {
            held: Source::Guard {
                base: base.to_path_buf(),
                manifest,
            },
        })
    }

    pub fn at(root: &Path, running: &str) -> Result<Self, String> {
        let marker = root.join(v2::POINTER);
        if marker.is_file() {
            let bytes = std::fs::read(&marker)
                .map_err(|error| format!("cannot read {}: {error}", marker.display()))?;
            let format = serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|held| held.get("format").and_then(serde_json::Value::as_u64));
            if format == Some(v3::FORMAT.into()) {
                return Self::modern(root, running, &bytes);
            }
            let text = String::from_utf8(bytes)
                .map_err(|error| format!("{} is not UTF-8: {error}", marker.display()))?;
            let pointer = v2::Pointer::parse(&text)?;
            let base = v2::local(root, &pointer.release, &pointer.snapshot.timestamp)?;
            let path = base.join(v2::LEAF);
            let body = std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            let manifest = v2::Manifest::parse(&body)?;
            pointer.bind(&manifest, body.as_bytes())?;
            return Self::load(&base, running, Width::Floor);
        }
        let seat = Seat::at(root)?;
        seat.supported(running)?;
        Ok(Self {
            held: Source::V1(seat),
        })
    }

    fn modern(root: &Path, running: &str, bytes: &[u8]) -> Result<Self, String> {
        let pointer = v3::Pointer::parse(bytes)?;
        if pointer.product != "plumb" || pointer.kind != v3::Kind::Configuration {
            return Err("the installed depot generation is not plumb configuration".into());
        }
        if pointer.version != running {
            return Err(format!(
                "depot configuration for {} requires that exact product binary, got {running}",
                pointer.version
            ));
        }
        let base = v3::local_generation(root, &pointer.generation)?;
        let path = base.join(v3::LEAF);
        let body = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let manifest = v3::Manifest::parse(&body)?;
        pointer.bind(&manifest, &body)?;
        let objects = manifest
            .objects
            .iter()
            .map(|held| Object {
                path: held.path.clone(),
                sha256: held.sha256.clone(),
                size: held.size,
            })
            .collect();
        Ok(Self {
            held: Source::V3 {
                base,
                manifest,
                generation: pointer.generation,
                objects,
            },
        })
    }

    pub fn staged(base: &Path, running: &str) -> Result<Self, String> {
        Self::load(base, running, Width::Exact)
    }

    fn load(base: &Path, running: &str, width: Width) -> Result<Self, String> {
        let path = base.join(v2::LEAF);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let manifest = v2::Manifest::parse(&text)?;
        if manifest.derivative != v2::Kind::Configuration || manifest.release.product != "plumb" {
            return Err("the staged depot snapshot is not plumb configuration".into());
        }
        let running = semver::Version::parse(running.trim_start_matches('v'))
            .map_err(|error| format!("cannot parse running Plumb version: {error}"))?;
        let released = semver::Version::parse(manifest.release.version.trim_start_matches('v'))
            .map_err(|error| format!("cannot parse depot release version: {error}"))?;
        let supported = match width {
            Width::Exact => running == released,
            Width::Floor => super::supports(&running, &released),
        };
        if !supported {
            let demand = match width {
                Width::Exact => "that exact product binary",
                Width::Floor => "that product version or a newer one",
            };
            return Err(format!(
                "depot configuration for {} requires {demand}, got {running}",
                manifest.release.version,
            ));
        }
        Ok(Self {
            held: Source::V2 {
                base: base.to_path_buf(),
                manifest,
            },
        })
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        match &self.held {
            Source::Guard { base, manifest } => manifest.read(base, path),
            Source::V1(seat) => seat.read(path),
            Source::V2 { base, manifest } => {
                anchored(path)?;
                let file = base.join(path);
                let bytes = std::fs::read(&file)
                    .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
                manifest.verify(path, &bytes)?;
                String::from_utf8(bytes).map_err(|error| format!("{path} is not UTF-8: {error}"))
            }
            Source::V3 { base, manifest, .. } => {
                anchored(path)?;
                let file = base.join(path);
                let bytes = std::fs::read(&file)
                    .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
                let executable = executable(&file)?;
                manifest.verify(path, &bytes, executable)?;
                String::from_utf8(bytes).map_err(|error| format!("{path} is not UTF-8: {error}"))
            }
            #[cfg(feature = "depot")]
            Source::Remote(held, _) => String::from_utf8(held.read(path)?)
                .map_err(|error| format!("{path} is not UTF-8: {error}")),
        }
    }

    pub fn mark(&self) -> &str {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.digest(),
            Source::V1(seat) => seat.mark(),
            Source::V2 { manifest, .. } => &manifest.snapshot.timestamp,
            Source::V3 { generation, .. } => generation,
            #[cfg(feature = "depot")]
            Source::Remote(held, _) => &held.generation,
        }
    }

    pub fn floor(&self) -> &str {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.target(),
            Source::V1(seat) => &seat.manifest().schema.version,
            Source::V2 { manifest, .. } => &manifest.release.version,
            Source::V3 { manifest, .. } => &manifest.version,
            #[cfg(feature = "depot")]
            Source::Remote(held, _) => &held.manifest.version,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match &self.held {
            Source::Guard { manifest, .. } => Some(manifest.target()),
            Source::V1(_) => None,
            Source::V2 { manifest, .. } => Some(&manifest.release.version),
            Source::V3 { manifest, .. } => Some(&manifest.version),
            #[cfg(feature = "depot")]
            Source::Remote(held, _) => Some(&held.manifest.version),
        }
    }

    pub fn objects(&self) -> &[super::Object] {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.objects(),
            Source::V1(seat) => &seat.manifest().objects,
            Source::V2 { manifest, .. } => &manifest.objects,
            Source::V3 { objects, .. } => objects,
            #[cfg(feature = "depot")]
            Source::Remote(_, objects) => objects,
        }
    }

    pub fn channel(&self) -> Option<&str> {
        match &self.held {
            Source::V3 { manifest, .. } => Some(&manifest.channel),
            #[cfg(feature = "depot")]
            Source::Remote(held, _) => Some(&held.manifest.channel),
            _ => None,
        }
    }
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .map(|held| held.permissions().mode() & 0o111 != 0)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn executable(path: &Path) -> Result<bool, String> {
    path.metadata()
        .map(|_| false)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))
}
