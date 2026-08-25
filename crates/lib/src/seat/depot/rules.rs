use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{Seat, anchored, v2};

pub struct Rules {
    held: Source,
}

enum Source {
    V1(Seat),
    V2 {
        base: PathBuf,
        manifest: v2::Manifest,
    },
}

static RULES: LazyLock<Result<Rules, String>> = LazyLock::new(Rules::open);

pub(super) fn held() -> Result<&'static Rules, String> {
    RULES.as_ref().map_err(Clone::clone)
}

impl Rules {
    fn open() -> Result<Self, String> {
        if let Some(root) = crate::config::value("PLUMB_DEPOT_SNAPSHOT") {
            return Self::staged(Path::new(&root), crate::version!("PLUMB"));
        }
        let seat = Seat::open()?;
        seat.supported(crate::version!("PLUMB"))?;
        Ok(Self {
            held: Source::V1(seat),
        })
    }

    pub fn staged(base: &Path, running: &str) -> Result<Self, String> {
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
        if running != released {
            return Err(format!(
                "depot configuration for {} requires that exact product binary, got {running}",
                manifest.release.version
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
}
