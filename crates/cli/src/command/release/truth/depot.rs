use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::shape::depot::Batch;
use crate::shape::release::{Format, Spec};

pub(in crate::command) struct Binding {
    pub release: plumb::depot::v2::Release,
    pub seal: super::record::Seal,
}

pub(in crate::command) struct Source<'a> {
    pub product: &'a str,
    pub authority: &'a str,
}

impl Source<'_> {
    pub fn latest(&self, channel: &str, binary: bool) -> Result<Binding, String> {
        let url = format!("{}/v1/channels/{channel}.json", self.authority);
        let pointer = super::verify::Surface(&url).pointer(channel)?;
        if pointer.product != self.product {
            return Err(format!(
                "{channel} pointer names {}, expected {}",
                pointer.product, self.product,
            ));
        }
        self.binding(&pointer.version, binary)
    }

    pub fn binding(&self, version: &str, binary: bool) -> Result<Binding, String> {
        let channel = super::super::channel(version)?;
        let url = format!(
            "{}/v1/releases/{channel}/{version}/seal.json",
            self.authority
        );
        let (seal, digest) = super::verify::Surface(&url).sealed(binary)?;
        if seal.product != self.product || seal.channel != channel || seal.version != version {
            return Err(format!(
                "release seal names {} {} {}, expected {} {channel} {version}",
                seal.product, seal.channel, seal.version, self.product
            ));
        }
        Ok(Binding {
            release: plumb::depot::v2::Release {
                product: seal.product.clone(),
                channel: seal.channel.clone(),
                version: seal.version.clone(),
                commit: seal.commit.clone(),
                seal: plumb::depot::v2::Seal {
                    url,
                    sha256: digest,
                },
            },
            seal,
        })
    }

    pub fn validator(&self, target: &str, binary: bool) -> Result<Binding, String> {
        let beta = self.latest("beta", binary)?;
        if super::compatibility::related(target, &beta.release.version).is_ok() {
            return Ok(beta);
        }
        let stable = self.latest("stable", binary)?;
        match super::compatibility::channel(target, &beta.release.version, &stable.release.version)?
        {
            super::compatibility::Channel::Beta => Ok(beta),
            super::compatibility::Channel::Stable => Ok(stable),
        }
    }
}

pub(in crate::command) struct Validated {
    pub version: String,
    pub artifact: String,
}

struct Validation<'a> {
    spec: &'a Spec,
    binding: &'a Binding,
    plan: &'a Batch,
}

pub(in crate::command) fn validate(
    spec: &Spec,
    binding: &Binding,
    plan: &Batch,
    recovery: Option<&Path>,
) -> Result<Validated, String> {
    let depot = spec
        .depot
        .as_ref()
        .ok_or_else(|| "release declares no depot".to_string())?;
    let binary = depot
        .validator
        .first()
        .ok_or_else(|| "configuration depot declares no validator".to_string())?;
    let target = native(spec)?;
    let artifact = binding.seal.artifacts.get(&target.key).ok_or_else(|| {
        format!(
            "release {} carries no {} binary artifact",
            binding.release.version, target.key
        )
    })?;
    let validation = Validation {
        spec,
        binding,
        plan,
    };
    if let Some(executable) = recovery {
        let artifact = super::compatibility::recovery(binding, executable, binary)?;
        return validation.run(executable, artifact);
    }
    let archive = super::verify::fetch(artifact)?;
    let result = (|| {
        let unpacked = tempfile::tempdir()
            .map_err(|error| format!("cannot stage the released validator: {error}"))?;
        unpack(&archive, unpacked.path(), target.format)?;
        let executable = locate(unpacked.path(), binary)?;
        validation.run(&executable, artifact.sha256.clone())
    })();
    let _ = std::fs::remove_file(archive);
    result
}

impl Validation<'_> {
    fn run(&self, executable: &Path, artifact: String) -> Result<Validated, String> {
        let depot = self
            .spec
            .depot
            .as_ref()
            .expect("validated depot declaration");
        let home = tempfile::tempdir()
            .map_err(|error| format!("cannot stage depot configuration: {error}"))?;
        let seat = home.path().join("depot");
        let snapshot = plumb::depot::v2::local(
            &seat,
            &self.plan.manifest.release,
            &self.plan.manifest.snapshot.timestamp,
        )?;
        for (path, bytes) in &self.plan.bodies {
            write(&snapshot.join(path), bytes)?;
        }
        let manifest = self.plan.manifest.encode()?;
        write(&snapshot.join(plumb::depot::v2::LEAF), manifest.as_bytes())?;
        let pointer = plumb::depot::v2::Pointer::new(&self.plan.manifest, manifest.as_bytes())?;
        write(
            &seat.join(plumb::depot::v2::POINTER),
            pointer.encode()?.as_bytes(),
        )?;
        let source = super::tree::Seat::open(&self.spec.root, &self.binding.release.commit)?;
        source.govern(self.plan)?;
        let mut validator = plumb::config::detached(executable);
        validator
            .args(depot.validator.iter().skip(1))
            .current_dir(source.path());
        for name in [
            "ROOT",
            "CHANNEL",
            "VERSION",
            "COMMIT",
            "TARGET",
            "ARTIFACTS",
            "OUTPUT",
            "CAPSULE",
            "PROMOTION",
            "ACTIVATED",
            "URL",
        ] {
            validator.env_remove(format!("{}_RELEASE_{name}", self.spec.environment()));
        }
        let output = validator
            .env_remove("PLUMB_HOME")
            .env_remove("PLUMB_GUARD_CONFIGURATION")
            .env("PLUMB_GUARD_DEPOT", &seat)
            .env("PLUMB_GUARD_VIEW", source.path())
            .env(
                format!("{}_RELEASE_VERSION", self.spec.environment()),
                &self.binding.release.version,
            )
            .env(
                format!("{}_DEPOT_SNAPSHOT", self.spec.environment()),
                &snapshot,
            )
            .output()
            .map_err(|error| {
                format!(
                    "cannot run released depot validator {}: {error}",
                    executable.display()
                )
            })?;
        if output.status.success() {
            return Ok(Validated {
                version: self.binding.release.version.clone(),
                artifact,
            });
        }
        Err(format!(
            "released {} validator refused the configuration:\n{}{}",
            self.binding.release.version,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn native(spec: &Spec) -> Result<&crate::shape::release::Target, String> {
    let triple = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else {
        return Err("the host has no released depot validator target".into());
    };
    spec.target(triple)
}

fn unpack(archive: &Path, destination: &Path, format: Format) -> Result<(), String> {
    match format {
        Format::Tar => {
            let file = File::open(archive)
                .map_err(|error| format!("cannot open {}: {error}", archive.display()))?;
            let gzip = flate2::read::GzDecoder::new(file);
            tar::Archive::new(gzip)
                .unpack(destination)
                .map_err(|error| format!("cannot unpack {}: {error}", archive.display()))
        }
        Format::Zip => {
            let file = File::open(archive)
                .map_err(|error| format!("cannot open {}: {error}", archive.display()))?;
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|error| format!("cannot open {}: {error}", archive.display()))?;
            for index in 0..zip.len() {
                extract(&mut zip, index, destination)?;
            }
            Ok(())
        }
    }
}

fn extract(
    zip: &mut zip::ZipArchive<File>,
    index: usize,
    destination: &Path,
) -> Result<(), String> {
    let mut entry = zip
        .by_index(index)
        .map_err(|error| format!("cannot read validator archive: {error}"))?;
    let Some(name) = entry.enclosed_name() else {
        return Err("validator archive carries an unanchored path".into());
    };
    let path = destination.join(name);
    if entry.is_dir() {
        return std::fs::create_dir_all(&path)
            .map_err(|error| format!("cannot create {}: {error}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read validator archive: {error}"))?;
    write(&path, &bytes)
}

fn locate(root: &Path, binary: &str) -> Result<PathBuf, String> {
    let wanted = if cfg!(windows) {
        format!("{binary}.exe")
    } else {
        binary.to_string()
    };
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        {
            let path = entry
                .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(&wanted) {
                return Ok(path);
            }
        }
    }
    Err(format!("released artifact carries no {wanted} validator"))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}
