use crate::shape::release::Spec;
use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Chart<'a> {
    spec: &'a Spec,
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

    pub fn publish(&self, version: &str, credential: &str) -> Result<String, String> {
        let Some(chart) = &self.spec.chart else {
            return Ok(format!("{} has no chart attachment", self.spec.product));
        };
        let identity = crate::command::ship::attachment::Identity {
            user: &chart.account,
            token: crate::command::ship::attachment::credential(credential)?,
        };
        self.package(version)?;
        let semver = release(version)?;
        let held = name(chart)?;
        let owner = owner(chart)?;
        self.login(&chart.registry, &identity)?;
        let archive = self.archive(&held, &semver);
        let seat = format!("oci://{}/{owner}/{held}", chart.registry);
        if let Some(carried) = self.carried(&seat, &semver)? {
            let held = crate::command::release::record::digest(&archive)?.0;
            if carried != held {
                return Err(format!(
                    "published chart drift: {seat} holds {carried} while this projection carries {held}"
                ));
            }
            return Ok(format!("{seat} already carries {version}"));
        }
        self.helm([
            "push",
            &archive.to_string_lossy(),
            &format!("oci://{}/{owner}", chart.registry),
        ])?;
        self.helm(["show", "chart", &seat, "--version", &semver.to_string()])?;
        Ok(format!("published chart attachment for {version}"))
    }

    fn stamp(&self, held: &str, version: &Version) -> Result<(), String> {
        let path = self.seat(held).join("Chart.yaml");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut lines = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("version:") {
                let _ = rest;
                lines.push(format!("version: {version}"));
            } else if let Some(rest) = line.strip_prefix("appVersion:") {
                let _ = rest;
                lines.push(format!("appVersion: \"{version}\""));
            } else {
                lines.push(line.to_string());
            }
        }
        lines.push(String::new());
        std::fs::write(&path, lines.join("\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    fn carried(&self, seat: &str, version: &Version) -> Result<Option<String>, String> {
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

    fn login(
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

    fn helm<const N: usize>(&self, args: [&str; N]) -> Result<(), String> {
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

    fn seat(&self, held: &str) -> PathBuf {
        self.spec.root.join("charts").join(held)
    }

    fn out(&self) -> PathBuf {
        self.spec.root.join("target/chart")
    }

    fn archive(&self, held: &str, version: &Version) -> PathBuf {
        self.out().join(format!("{held}-{version}.tgz"))
    }
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

fn owner(chart: &crate::shape::release::Chart) -> Result<String, String> {
    seat(chart, 0)
}

fn name(chart: &crate::shape::release::Chart) -> Result<String, String> {
    seat(chart, 1)
}

fn seat(chart: &crate::shape::release::Chart, index: usize) -> Result<String, String> {
    chart
        .chart
        .split('/')
        .nth(index)
        .filter(|held| !held.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "chart attachment must name one owner and one chart".to_string())
}

fn release(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("release version is not semantic: {error}"))
}
