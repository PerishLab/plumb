use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::shape::depot::Batch;
use crate::shape::release::{Format, Spec};

pub(in crate::command) struct Binding {
    pub release: plumb::depot::v2::Release,
    pub seal: super::record::Seal,
}

pub(in crate::command) struct Source<'a>(pub &'a Spec);

impl Source<'_> {
    pub fn binding(&self, version: &str, binary: bool) -> Result<Binding, String> {
        let spec = self.0;
        let channel = super::super::channel(version)?;
        let url = format!(
            "{}/v1/releases/{channel}/{version}/seal.json",
            spec.authority
        );
        let (seal, digest) = super::verify::Surface(&url).sealed(binary)?;
        if seal.product != spec.product || seal.channel != channel || seal.version != version {
            return Err(format!(
                "release seal names {} {} {}, expected {} {channel} {version}",
                seal.product, seal.channel, seal.version, spec.product
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

    pub fn current(&self, release: &plumb::depot::v2::Release) -> Result<bool, String> {
        super::verify::active(&self.0.authority, release)
    }
}

pub(in crate::command) fn validate(
    spec: &Spec,
    binding: &Binding,
    plan: &Batch,
) -> Result<(), String> {
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
    let archive = super::verify::fetch(artifact)?;
    let result = (|| {
        let unpacked = tempfile::tempdir()
            .map_err(|error| format!("cannot stage the released validator: {error}"))?;
        unpack(&archive, unpacked.path(), target.format)?;
        let executable = locate(unpacked.path(), binary)?;

        let snapshot = tempfile::tempdir()
            .map_err(|error| format!("cannot stage depot configuration: {error}"))?;
        for (path, bytes) in &plan.bodies {
            write(&snapshot.path().join(path), bytes)?;
        }
        write(
            &snapshot.path().join(plumb::depot::v2::LEAF),
            plan.manifest.encode()?.as_bytes(),
        )?;

        let output = Command::new(&executable)
            .args(depot.validator.iter().skip(1))
            .current_dir(&spec.root)
            .env(
                format!("{}_DEPOT_SNAPSHOT", spec.environment()),
                snapshot.path(),
            )
            .output()
            .map_err(|error| {
                format!(
                    "cannot run released depot validator {}: {error}",
                    executable.display()
                )
            })?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "released {} validator refused the configuration:\n{}{}",
            binding.release.version,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    })();
    let _ = std::fs::remove_file(archive);
    result
}

fn native(spec: &Spec) -> Result<&crate::shape::release::Target, String> {
    let triple = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
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
