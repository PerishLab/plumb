use crate::shape::release::Spec;
use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Chart<'a> {
    pub(in crate::command::ship) spec: &'a Spec,
}

pub fn chart(spec: &Spec) -> Chart<'_> {
    Chart { spec }
}

impl Chart<'_> {
    pub(in crate::command) fn prepare(&self, version: &str) -> Result<(), String> {
        let Some(chart) = &self.spec.chart else {
            return Ok(());
        };
        self.stamp(&name(chart)?, &release(version)?)
    }

    pub fn package(&self, version: &str) -> Result<String, String> {
        let Some(chart) = &self.spec.chart else {
            return Ok(format!("{} has no chart attachment", self.spec.product));
        };
        let identity = release(version)?;
        self.stamp(&name(chart)?, &identity)?;
        let out = self.out();
        std::fs::create_dir_all(&out)
            .map_err(|error| format!("cannot open {}: {error}", out.display()))?;
        self.helm([
            "package",
            &self.seat(&name(chart)?).to_string_lossy(),
            "--destination",
            &out.to_string_lossy(),
        ])?;
        let archive = self.archive(&name(chart)?, &identity);
        if !archive.is_file() {
            return Err(format!("chart attachment left no {}", archive.display()));
        }
        normalize(&archive)?;
        Ok(format!("packaged chart attachment for {version}"))
    }

    pub fn exact(&self, version: &str, credential: &str, reuse: &str) -> Result<String, String> {
        crate::command::ship::package::chart::run(self, version, credential, reuse)
    }

    fn stamp(&self, held: &str, version: &Version) -> Result<(), String> {
        let path = self.seat(held).join("Chart.yaml");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        std::fs::write(&path, stamped(&text, version))
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    pub(in crate::command::ship) fn carried(
        &self,
        seat: &str,
        version: &Version,
    ) -> Result<Option<String>, String> {
        let out = tempfile::tempdir()
            .map_err(|error| format!("cannot open a chart readback seat: {error}"))?;
        let pulled = Command::new("helm")
            .args([
                "pull",
                seat,
                "--version",
                &version.to_string(),
                "--destination",
                &out.path().to_string_lossy(),
            ])
            .current_dir(&self.spec.root)
            .output()
            .map_err(|error| format!("cannot run helm: {error}"))?;
        if !pulled.status.success() {
            return Ok(None);
        }
        let mut held = None;
        for entry in std::fs::read_dir(out.path())
            .map_err(|error| format!("cannot read the chart readback seat: {error}"))?
            .flatten()
        {
            if entry.path().extension().is_some_and(|held| held == "tgz") {
                held = Some(entry.path());
            }
        }
        let path = held.ok_or_else(|| format!("{seat} answered with no chart archive"))?;
        Ok(Some(crate::command::release::record::digest(&path)?.0))
    }

    pub(in crate::command::ship) fn login(
        &self,
        registry: &str,
        identity: &crate::command::ship::attachment::Identity<'_>,
    ) -> Result<(), String> {
        let mut child = Command::new("helm")
            .args([
                "registry",
                "login",
                registry,
                "--username",
                identity.user,
                "--password-stdin",
            ])
            .current_dir(&self.spec.root)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot run helm: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "helm login refused its stdin".to_string())?
            .write_all(identity.token.as_bytes())
            .map_err(|error| format!("cannot send registry token: {error}"))?;
        let status = child
            .wait()
            .map_err(|error| format!("cannot run helm: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("chart attachment login failed".into())
        }
    }

    pub(in crate::command::ship) fn helm<const N: usize>(
        &self,
        args: [&str; N],
    ) -> Result<(), String> {
        let status = Command::new("helm")
            .args(args)
            .current_dir(&self.spec.root)
            .status()
            .map_err(|error| format!("cannot run helm: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("chart attachment command failed".into())
        }
    }

    pub(in crate::command::ship) fn seat(&self, held: &str) -> PathBuf {
        self.spec.root.join("charts").join(held)
    }

    fn out(&self) -> PathBuf {
        self.spec.root.join("target/chart")
    }

    pub(in crate::command::ship) fn archive(&self, held: &str, version: &Version) -> PathBuf {
        self.out().join(format!("{held}-{version}.tgz"))
    }
}

pub(in crate::command::ship) fn stamped(text: &str, version: &Version) -> String {
    let mut lines = text
        .lines()
        .map(|line| {
            if line.starts_with("version:") {
                format!("version: {version}")
            } else if line.starts_with("appVersion:") {
                format!("appVersion: \"{version}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();
    lines.push(String::new());
    lines.join("\n")
}

pub(in crate::command::ship) fn owner(
    chart: &crate::shape::release::Chart,
) -> Result<String, String> {
    identity(chart, 0)
}

pub(in crate::command::ship) fn name(
    chart: &crate::shape::release::Chart,
) -> Result<String, String> {
    identity(chart, 1)
}

fn identity(chart: &crate::shape::release::Chart, index: usize) -> Result<String, String> {
    chart
        .chart
        .split('/')
        .nth(index)
        .filter(|held| !held.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "chart attachment must name one owner and one chart".to_string())
}

struct Entry {
    path: PathBuf,
    kind: tar::EntryType,
    mode: u32,
    body: Vec<u8>,
}

fn normalize(path: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let mut entries = Vec::new();
    for held in archive
        .entries()
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
    {
        let mut held = held.map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let kind = held.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err(format!(
                "chart archive {} carries unsupported entry {}",
                path.display(),
                held.path()
                    .map_err(|error| format!("cannot read chart path: {error}"))?
                    .display()
            ));
        }
        let mode = held
            .header()
            .mode()
            .map_err(|error| format!("cannot read chart entry mode: {error}"))?;
        let entry = held
            .path()
            .map_err(|error| format!("cannot read chart path: {error}"))?
            .into_owned();
        let mut body = Vec::new();
        held.read_to_end(&mut body)
            .map_err(|error| format!("cannot read chart entry {}: {error}", entry.display()))?;
        entries.push(Entry {
            path: entry,
            kind,
            mode,
            body,
        });
    }
    drop(archive);

    let file = File::create(path)
        .map_err(|error| format!("cannot rewrite {}: {error}", path.display()))?;
    let gzip = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(gzip);
    for entry in entries {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(entry.kind);
        header.set_mode(entry.mode);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_size(entry.body.len() as u64);
        header.set_cksum();
        archive
            .append_data(&mut header, &entry.path, entry.body.as_slice())
            .map_err(|error| format!("cannot normalize {}: {error}", path.display()))?;
    }
    let gzip = archive
        .into_inner()
        .map_err(|error| format!("cannot finish {}: {error}", path.display()))?;
    gzip.finish()
        .map_err(|error| format!("cannot finish {}: {error}", path.display()))?;
    Ok(())
}

pub(in crate::command::ship) fn release(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("release version is not semantic: {error}"))
}
