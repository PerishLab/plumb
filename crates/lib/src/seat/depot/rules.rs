use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{Seat, anchored, v2};

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
        if let Some(root) = crate::config::value("PLUMB_GUARD_CONFIGURATION") {
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
            let text = std::fs::read_to_string(&marker)
                .map_err(|error| format!("cannot read {}: {error}", marker.display()))?;
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
        }
    }

    pub fn mark(&self) -> &str {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.digest(),
            Source::V1(seat) => seat.mark(),
            Source::V2 { manifest, .. } => &manifest.snapshot.timestamp,
        }
    }

    pub fn floor(&self) -> &str {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.target(),
            Source::V1(seat) => &seat.manifest().schema.version,
            Source::V2 { manifest, .. } => &manifest.release.version,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match &self.held {
            Source::Guard { manifest, .. } => Some(manifest.target()),
            Source::V1(_) => None,
            Source::V2 { manifest, .. } => Some(&manifest.release.version),
        }
    }

    pub fn objects(&self) -> &[super::Object] {
        match &self.held {
            Source::Guard { manifest, .. } => manifest.objects(),
            Source::V1(seat) => &seat.manifest().objects,
            Source::V2 { manifest, .. } => &manifest.objects,
        }
    }
}
