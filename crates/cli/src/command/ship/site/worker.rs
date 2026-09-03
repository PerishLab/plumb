use super::model::App;
use super::process::Call;
use crate::shape::release::{Cfworker, Spec};
use plumb::rig::Site;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Seat<'a> {
    pub root: &'a Path,
    pub version: &'a str,
}

impl Seat<'_> {
    pub fn exact(&self, reuse: &str) -> Result<String, String> {
        let (spec, held) = self.declared()?;
        let app = App::read(self.root)?;
        let site = self.vantage(&spec, &held)?;
        previews(&site, &app.worker)?;
        let source = crate::command::ship::package::project::Source::parse(reuse)?;
        match source.kind.as_str() {
            "none" => self.build(&app)?,
            "workload" => restore(
                &app,
                &crate::command::ship::package::project::fetch("worker", &source.source)?,
            )?,
            "url" => return Err("a held publication URL must skip the worker action".into()),
            _ => unreachable!(),
        }
        if !app.index().is_file() {
            return Err(format!(
                "worker workload produced no {}",
                app.index().display()
            ));
        }
        let workload = bundle(&app)?;
        let publication = self.stage(&app, &site)?;
        serde_json::to_string(&serde_json::json!({
            "format": "plumb.worker-project/v1",
            "worker": app.worker,
            "version": self.version,
            "workload": workload,
            "publication": publication.url,
            "depot": {
                "schema": "plumb.depot-worker/v1",
                "marker": self.version,
                "worker": app.worker,
                "version": publication.id,
            },
        }))
        .map_err(|error| format!("cannot encode worker project: {error}"))
    }

    fn stage(&self, app: &App, site: &Site) -> Result<Publication, String> {
        let raw = super::process::held(Call {
            bin: "pnpm",
            args: &[
                "exec",
                "wrangler",
                "versions",
                "upload",
                "--tag",
                self.version,
            ],
            cwd: &app.seat,
            env: &[
                ("CLOUDFLARE_ACCOUNT_ID", &site.account),
                ("CLOUDFLARE_API_TOKEN", &site.token),
            ],
        })
        .map_err(|error| format!("cannot upload a worker version: {error}"))?;
        let version = stamped(&raw)?;
        let url = format!("https://{}-{}.{}", &version[..8], app.worker, root(site)?);
        super::reach::prove(&url, site)?;
        Ok(Publication { id: version, url })
    }

    fn build(&self, app: &App) -> Result<(), String> {
        let marks = super::artifact::marks(app)?;
        super::process::run(Call {
            bin: "pnpm",
            args: &["--filter", &app.package, "build"],
            cwd: &app.root,
            env: &[
                ("BUILD_COMMIT", &marks.commit),
                ("BUILD_VERSION", &marks.version),
            ],
        })?;
        if app.index().is_file() {
            Ok(())
        } else {
            Err(format!("build produced no {}", app.index().display()))
        }
    }

    fn declared(&self) -> Result<(Spec, Cfworker), String> {
        let spec = Spec::resolve(self.root)?;
        let held = spec
            .cfworker
            .clone()
            .ok_or_else(|| "this repository declares no worker attachment".to_string())?;
        Ok((spec, held))
    }

    fn vantage(&self, spec: &Spec, held: &Cfworker) -> Result<Site, String> {
        let mut site = super::settings::read()?;
        site.account = held.account.clone();
        site.domain = held.domain.clone();
        let _ = spec;
        if site.token.is_empty() {
            return Err("PLUMB_SITE_TOKEN is required to project onto a worker".into());
        }
        Ok(site)
    }
}

struct Publication {
    id: String,
    url: String,
}

pub(in crate::command) fn deploy(
    root: &Path,
    expected: &str,
    version: &str,
) -> Result<String, String> {
    let spec = Spec::resolve(root)?;
    let held = spec
        .cfworker
        .ok_or_else(|| "this repository declares no worker attachment".to_string())?;
    let app = App::read(root)?;
    if app.worker != expected {
        return Err(format!(
            "worker projection names {expected}, not declared {}",
            app.worker
        ));
    }
    let mut site = super::settings::read()?;
    site.account = held.account;
    site.domain = held.domain;
    super::settings::require(&site)?;
    let target = format!("{version}@100%");
    super::process::run(Call {
        bin: "pnpm",
        args: &[
            "exec",
            "wrangler",
            "versions",
            "deploy",
            &target,
            "--name",
            &app.worker,
            "-y",
        ],
        cwd: &app.seat,
        env: &[
            ("CLOUDFLARE_ACCOUNT_ID", &site.account),
            ("CLOUDFLARE_API_TOKEN", &site.token),
        ],
    })?;
    super::reach::prove(&format!("https://{}/", site.domain), &site)?;
    Ok(format!(
        "deployed worker version {version} at https://{}/",
        site.domain
    ))
}

fn bundle(app: &App) -> Result<PathBuf, String> {
    let mut members = BTreeMap::new();
    collect(&app.dist, &app.dist, &mut members)?;
    if members.is_empty() {
        return Err(format!("worker workload is empty: {}", app.dist.display()));
    }
    let seat = app.root.join("target/cfworker");
    std::fs::create_dir_all(&seat)
        .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
    let path = seat.join(format!("{}-worker.tar.gz", app.worker));
    crate::command::ship::archive::bundle(&path, &members)?;
    Ok(path)
}

fn collect(
    root: &Path,
    seat: &Path,
    members: &mut BTreeMap<PathBuf, crate::command::ship::archive::Member>,
) -> Result<(), String> {
    let mut entries = std::fs::read_dir(seat)
        .map_err(|error| format!("cannot read {}: {error}", seat.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot read {}: {error}", seat.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if kind.is_dir() {
            collect(root, &path, members)?;
        } else if kind.is_file() {
            let name = path
                .strip_prefix(root)
                .map_err(|error| format!("cannot name {}: {error}", path.display()))?
                .to_path_buf();
            members.insert(
                name,
                crate::command::ship::archive::Member {
                    bytes: std::fs::read(&path)
                        .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
                    mode: 0o644,
                },
            );
        } else {
            return Err(format!(
                "worker workload carries unsupported {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn restore(app: &App, bytes: &[u8]) -> Result<(), String> {
    let mut inspected = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in inspected
        .entries()
        .map_err(|error| format!("cannot read reusable worker workload: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("cannot read reusable worker workload: {error}"))?;
        if !entry.header().entry_type().is_file() {
            return Err("reusable worker workload carries a non-file entry".into());
        }
    }
    std::fs::create_dir_all(&app.dist)
        .map_err(|error| format!("cannot open {}: {error}", app.dist.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    archive
        .unpack(&app.dist)
        .map_err(|error| format!("cannot restore reusable worker workload: {error}"))
}

fn previews(site: &Site, worker: &str) -> Result<(), String> {
    let seat = format!("workers/scripts/{worker}/subdomain");
    let held = super::cloud::Vantage::new(site).get(&seat)?;
    if held.get("previews_enabled").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    Err(format!("worker {worker} refuses to show version previews"))
}

fn root(site: &Site) -> Result<String, String> {
    let held = super::cloud::Vantage::new(site).get("workers/subdomain")?;
    held.get("subdomain")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|value| format!("{value}.workers.dev"))
        .ok_or_else(|| "this account owns no workers.dev subdomain".to_string())
}

fn stamped(raw: &str) -> Result<String, String> {
    raw.lines()
        .find_map(|line| line.split_once("Worker Version ID:"))
        .map(|(_, held)| held.trim().to_string())
        .filter(|held| held.len() >= 8)
        .ok_or_else(|| "wrangler named no worker version".to_string())
}
