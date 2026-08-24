use crate::shape::release::Spec;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Asset {
    pub key: String,
    pub file: String,
    pub mime: String,
    pub source: Option<PathBuf>,
}

pub fn list(spec: &Spec) -> Result<Vec<Asset>, String> {
    let mut assets = Vec::new();
    if spec.skill {
        assets.push(asset(
            "skill",
            format!("{}-skill.tar.gz", spec.product),
            "application/gzip",
        ));
    }
    if spec.deb.is_some() {
        assets.push(asset(
            "linux-x64-deb",
            format!("{}-x86_64-unknown-linux-gnu.deb", spec.product),
            "application/vnd.debian.binary-package",
        ));
    }
    Ok(assets)
}

fn asset(key: &str, file: String, mime: &str) -> Asset {
    Asset {
        key: key.into(),
        file,
        mime: mime.into(),
        source: None,
    }
}
