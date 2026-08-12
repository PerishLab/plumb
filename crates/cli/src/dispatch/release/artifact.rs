use super::model::Spec;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Asset {
    pub key: String,
    pub file: String,
    pub mime: String,
    pub source: Option<PathBuf>,
}

pub fn list(spec: &Spec, version: &str) -> Result<Vec<Asset>, String> {
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
    let mut keys = spec
        .target
        .iter()
        .map(|target| target.key.clone())
        .chain(assets.iter().map(|asset| asset.key.clone()))
        .collect::<BTreeSet<_>>();
    let mut files = spec
        .target
        .iter()
        .map(|target| target.archive.clone())
        .chain(assets.iter().map(|asset| asset.file.clone()))
        .collect::<BTreeSet<_>>();
    for path in crate::shape::changelog::artifacts(&spec.root, version)? {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("artifact shape should prove one UTF-8 name")
            .to_string();
        if !keys.insert(name.clone()) || !files.insert(name.clone()) {
            return Err(format!(
                "version artifact collides with release artifact: {name}"
            ));
        }
        assets.push(source(&name, path));
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

fn source(name: &str, path: PathBuf) -> Asset {
    Asset {
        key: name.into(),
        file: name.into(),
        mime: "application/octet-stream".into(),
        source: Some(path),
    }
}
