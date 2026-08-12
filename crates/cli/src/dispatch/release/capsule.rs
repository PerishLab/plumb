use super::manager;
use super::model::{Format, Spec};
use super::proof;
pub use super::record::{Capsule, Local, Pointer};
use super::record::{Remote, Seal, digest, json};
use super::{artifact, generator};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub struct Compile<'a> {
    pub spec: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub artifacts: &'a Path,
    pub out: &'a Path,
    pub promotion: Option<&'a Path>,
}

struct Draft<'a> {
    root: &'a Path,
    authority: &'a str,
}

struct Stable<'a> {
    spec: &'a Spec,
    version: &'a str,
    commit: &'a str,
    managers: &'a Path,
    seal: &'a Local,
}

struct Route {
    key: String,
    url: String,
    mime: String,
}

pub fn compile(input: Compile<'_>) -> Result<String, String> {
    manager::intent(input.channel, input.version)?;
    proof::commit(input.commit)?;
    let spec = Spec::read(input.spec)?;
    if input.out.exists() {
        return Err(format!(
            "capsule output already exists: {}",
            input.out.display()
        ));
    }
    let seat = input.out.join("managers");
    manager::write(input.spec, input.channel, input.version, &seat)?;
    let assets = artifact::list(&spec, input.version)?;
    let declared = declared(&spec, &assets);
    complete(input.artifacts, &declared)?;
    let payload = input.out.join("payload");
    std::fs::create_dir(&payload)
        .map_err(|error| format!("cannot create {}: {error}", payload.display()))?;

    let mut objects = Vec::new();
    let mut artifacts = BTreeMap::new();
    let draft = Draft {
        root: input.out,
        authority: &spec.authority,
    };
    for target in &spec.target {
        let content = match target.format {
            Format::Tar => "application/gzip",
            Format::Zip => "application/zip",
        };
        let source = stage(input.artifacts, &payload, &target.archive)?;
        let local = draft.object(source, &target.archive, content)?;
        artifacts.insert(target.key.clone(), local.remote.clone());
        objects.push(local);
    }
    for asset in assets {
        let source = stage(input.artifacts, &payload, &asset.file)?;
        let local = draft.object(source, &asset.file, &asset.mime)?;
        artifacts.insert(asset.key.clone(), local.remote.clone());
        objects.push(local);
    }

    let mut managers = BTreeMap::new();
    for (kind, name, content) in [
        ("unix", "manage.sh", "text/x-shellscript; charset=utf-8"),
        ("windows", "manage.ps1", "text/plain; charset=utf-8"),
    ] {
        let path = seat.join(name);
        if path.is_file() {
            let local = draft.object(path, name, content)?;
            managers.insert(kind.to_string(), local.remote.clone());
            objects.push(local);
        }
    }

    let promotion = proof::promotion(proof::Claim {
        spec: &spec,
        channel: input.channel,
        version: input.version,
        commit: input.commit,
        path: input.promotion,
    })?;
    let url = format!(
        "{}/v1/releases/{}/{}/seal.json",
        spec.authority, input.channel, input.version
    );
    let seal = Seal {
        schema: 1,
        product: spec.product.clone(),
        channel: input.channel.into(),
        version: input.version.into(),
        commit: input.commit.into(),
        url: url.clone(),
        generator: generator::resolve(generator::Claim {
            spec: &spec,
            channel: input.channel,
            version: input.version,
            commit: input.commit,
            promotion: input.promotion,
        })?,
        artifacts,
        managers,
        proof: promotion,
        radius: None,
    };
    let path = input.out.join("seal.json");
    json(&path, &seal)?;
    let local = draft.local(
        &path,
        Route {
            key: format!("v1/releases/{}/{}/seal.json", input.channel, input.version),
            url,
            mime: "application/json; charset=utf-8".into(),
        },
    )?;

    let (roots, pointer) = if input.channel == "stable" {
        draft.stable(Stable {
            spec: &spec,
            version: input.version,
            commit: input.commit,
            managers: &seat,
            seal: &local,
        })?
    } else {
        (Vec::new(), None)
    };
    let capsule = Capsule {
        schema: 1,
        product: spec.product,
        channel: input.channel.into(),
        version: input.version.into(),
        authority: spec.authority,
        objects,
        seal: local,
        roots,
        pointer,
    };
    json(&input.out.join("capsule.json"), &capsule)?;
    Ok(format!(
        "compiled {} {} capsule in {}",
        input.channel,
        input.version,
        input.out.display()
    ))
}

impl Draft<'_> {
    fn stable(&self, input: Stable<'_>) -> Result<(Vec<Local>, Option<Local>), String> {
        let mut locals = Vec::new();
        let mut managers = BTreeMap::new();
        for (kind, name, mime) in [
            ("unix", "manage.sh", "text/x-shellscript; charset=utf-8"),
            ("windows", "manage.ps1", "text/plain; charset=utf-8"),
        ] {
            let path = input.managers.join("canonical").join(name);
            if !path.is_file() {
                continue;
            }
            let remote = self.public(path, name, mime)?;
            managers.insert(kind.into(), remote.remote.clone());
            locals.push(remote);
        }
        let pointer = Pointer {
            schema: 1,
            product: input.spec.product.clone(),
            channel: "stable".into(),
            version: input.version.into(),
            commit: input.commit.into(),
            seal: input.seal.remote.clone(),
            managers,
        };
        let path = self.root.join("stable.json");
        json(&path, &pointer)?;
        let local = self.local(
            &path,
            Route {
                key: "v1/channels/stable.json".into(),
                url: format!("{}/v1/channels/stable.json", self.authority),
                mime: "application/json; charset=utf-8".into(),
            },
        )?;
        Ok((locals, Some(local)))
    }

    fn object(&self, path: PathBuf, name: &str, mime: &str) -> Result<Local, String> {
        let (digest, size) = digest(&path)?;
        let key = format!("v1/objects/sha256/{digest}/{name}");
        self.local(
            &path,
            Route {
                url: format!("{}/{key}", self.authority),
                key,
                mime: mime.into(),
            },
        )
        .map(|mut held| {
            held.remote.sha256 = digest;
            held.remote.size = size;
            held
        })
    }

    fn public(&self, path: PathBuf, name: &str, mime: &str) -> Result<Local, String> {
        self.local(
            &path,
            Route {
                key: name.into(),
                url: format!("{}/{name}", self.authority),
                mime: mime.into(),
            },
        )
    }

    fn local(&self, path: &Path, route: Route) -> Result<Local, String> {
        let (sha256, size) = digest(path)?;
        let source = path
            .strip_prefix(self.root)
            .map_err(|_| format!("{} is outside {}", path.display(), self.root.display()))?
            .to_string_lossy()
            .to_string();
        let name = path
            .file_name()
            .ok_or_else(|| format!("object has no name: {}", path.display()))?
            .to_string_lossy()
            .to_string();
        Ok(Local {
            source,
            key: route.key,
            remote: Remote {
                name,
                mime: route.mime,
                sha256,
                size,
                url: route.url,
            },
        })
    }
}

fn declared(spec: &Spec, assets: &[artifact::Asset]) -> BTreeSet<String> {
    spec.target
        .iter()
        .map(|held| held.archive.clone())
        .chain(assets.iter().map(|held| held.file.clone()))
        .collect()
}

fn complete(root: &Path, declared: &BTreeSet<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("cannot read artifact root {}: {error}", root.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read artifact entry: {error}"))?;
        if !entry.path().is_file() {
            return Err(format!(
                "artifact root contains non-file {}",
                entry.path().display()
            ));
        }
        found.insert(entry.file_name().to_string_lossy().to_string());
    }
    if &found != declared {
        return Err(format!(
            "artifact set disagrees: expected {declared:?}, found {found:?}"
        ));
    }
    Ok(())
}

fn stage(from: &Path, to: &Path, name: &str) -> Result<PathBuf, String> {
    let source = from.join(name);
    let target = to.join(name);
    std::fs::copy(&source, &target).map_err(|error| {
        format!(
            "cannot stage artifact {} as {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(target)
}
