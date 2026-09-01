use super::record::{Local, Pointer, Remote as Record};
use super::storage::{Authority, Remote};
use crate::shape::release::Spec;
use std::collections::BTreeMap;
use std::path::Path;

pub fn managers(
    spec: &Spec,
    marker: &crate::command::release::ReleaseMarker,
    authority: &impl Authority,
) -> Result<String, String> {
    Project { spec }.managers(marker, authority)
}

pub fn channel(
    spec: &Spec,
    marker: &crate::command::release::ReleaseMarker,
    authority: &impl Authority,
) -> Result<String, String> {
    Project { spec }.channel(marker, authority)
}

struct Project<'a> {
    spec: &'a Spec,
}

impl Project<'_> {
    fn managers(
        &self,
        marker: &crate::command::release::ReleaseMarker,
        authority: &impl Authority,
    ) -> Result<String, String> {
        if marker.channel != "stable" {
            return Err("only stable may activate managers".into());
        }
        let stage = tempfile::tempdir()
            .map_err(|error| format!("cannot stage stable managers: {error}"))?;
        let root = stage.path().join("managers");
        super::manager::write(
            &self.spec.manifest(),
            &marker.channel,
            &marker.marker,
            &root,
        )?;
        let roots = self.roots(&root)?;
        let remote = Remote::new(authority)?;
        for object in &roots {
            remote.shift(object, &root)?;
            super::verify::object(object)?;
        }
        Ok(format!("activated stable managers for {}", marker.marker))
    }

    fn channel(
        &self,
        marker: &crate::command::release::ReleaseMarker,
        authority: &impl Authority,
    ) -> Result<String, String> {
        let stage = tempfile::tempdir()
            .map_err(|error| format!("cannot stage channel projection: {error}"))?;
        let source = format!(
            "{}/v1/releases/{}/{}/seal.json",
            marker.authority, marker.channel, marker.marker
        );
        let seal =
            super::verify::describe(&source, "seal.json", "application/json; charset=utf-8")?;
        let pointer = Pointer {
            schema: 1,
            product: marker.product.clone(),
            channel: marker.channel.clone(),
            version: marker.marker.clone(),
            commit: marker.commit.clone(),
            seal,
            managers: self.pointers(marker, stage.path())?,
        };
        let path = stage.path().join(format!("{}.json", marker.channel));
        super::record::json(&path, &pointer)?;
        let route = Route {
            key: format!("v1/channels/{}.json", marker.channel),
            url: format!("{}/v1/channels/{}.json", marker.authority, marker.channel),
            mime: "application/json; charset=utf-8",
        };
        let local = local(stage.path(), &path, route)?;
        Remote::new(authority)?.activate(&local, stage.path())?;
        super::verify::object(&local)?;
        Ok(format!(
            "activated {} channel for {}",
            marker.channel, marker.marker
        ))
    }

    fn pointers(
        &self,
        marker: &crate::command::release::ReleaseMarker,
        stage: &Path,
    ) -> Result<BTreeMap<String, Record>, String> {
        if marker.channel != "stable" {
            return Ok(BTreeMap::new());
        }
        let root = stage.join("managers");
        super::manager::write(
            &self.spec.manifest(),
            &marker.channel,
            &marker.marker,
            &root,
        )?;
        Ok(self
            .roots(&root)?
            .into_iter()
            .map(|root| (kind(&root.remote.name), root.remote))
            .collect())
    }

    fn roots(&self, stage: &Path) -> Result<Vec<Local>, String> {
        let mut held = Vec::new();
        for (name, mime) in [
            ("manage.sh", "text/x-shellscript; charset=utf-8"),
            ("manage.ps1", "text/plain; charset=utf-8"),
        ] {
            let path = stage.join("canonical").join(name);
            if path.is_file() {
                held.push(local(
                    stage,
                    &path,
                    Route {
                        key: name.to_string(),
                        url: format!("{}/{name}", self.spec.authority),
                        mime,
                    },
                )?);
            }
        }
        Ok(held)
    }
}

struct Route<'a> {
    key: String,
    url: String,
    mime: &'a str,
}

fn local(root: &Path, path: &Path, route: Route<'_>) -> Result<Local, String> {
    let (sha256, size) = super::record::digest(path)?;
    let source = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?
        .to_string_lossy()
        .to_string();
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} has no name", path.display()))?
        .to_string_lossy()
        .to_string();
    Ok(Local {
        source,
        key: route.key,
        remote: Record {
            name,
            mime: route.mime.to_string(),
            sha256,
            size,
            url: route.url,
        },
    })
}

fn kind(name: &str) -> String {
    if name.ends_with(".ps1") {
        "windows".into()
    } else {
        "unix".into()
    }
}
