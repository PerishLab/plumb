use super::model::App;
use super::process::Call;
use crate::shape::release::{Cfworker, Spec};
use plumb::rig::Site;
use serde_json::{Value, json};
use std::path::Path;

pub struct Seat<'a> {
    pub root: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
}

impl Seat<'_> {
    pub fn rehearse(&self) -> Result<String, String> {
        let (spec, held) = self.declared()?;
        let app = App::read(self.root)?;
        let site = self.vantage(&spec, &held)?;
        previews(&site, &app.worker)?;
        Ok(format!(
            "worker {} shows versions at {}",
            app.worker,
            root(&site)?
        ))
    }

    pub fn publish(&self) -> Result<String, String> {
        let (spec, held) = self.declared()?;
        let app = App::read(self.root)?;
        let site = self.vantage(&spec, &held)?;
        previews(&site, &app.worker)?;
        self.build(&app)?;
        if self.channel == "stable" {
            return self.settle(&app, &site, &held);
        }
        self.stage(&app, &site)
    }

    fn stage(&self, app: &App, site: &Site) -> Result<String, String> {
        let raw = super::process::held(Call {
            bin: "pnpm",
            args: &["exec", "wrangler", "versions", "upload"],
            cwd: &app.seat,
            env: &[
                ("CLOUDFLARE_ACCOUNT_ID", &site.account),
                ("CLOUDFLARE_API_TOKEN", &site.token),
            ],
        })
        .map_err(|error| format!("cannot upload a worker version: {error}"))?;
        let version = stamped(&raw)?;
        let url = format!("https://{}-{}.{}", &version[..8], app.worker, root(site)?);
        reachable(&url, site)?;
        Ok(format!("staged {} {} at {url}", app.worker, self.version))
    }

    fn settle(&self, app: &App, site: &Site, held: &Cfworker) -> Result<String, String> {
        super::process::run(Call {
            bin: "pnpm",
            args: &["exec", "wrangler", "deploy", "--domain", &held.domain],
            cwd: &app.seat,
            env: &[
                ("CLOUDFLARE_ACCOUNT_ID", &held.account),
                ("CLOUDFLARE_API_TOKEN", &site.token),
            ],
        })?;
        reachable(&format!("https://{}/", held.domain), site)?;
        Ok(format!(
            "settled {} {} on {}",
            app.worker, self.version, held.domain
        ))
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
        let spec = Spec::read(&self.root.join("plumb.toml"))?;
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

fn previews(site: &Site, worker: &str) -> Result<(), String> {
    let seat = format!("workers/scripts/{worker}/subdomain");
    let held = super::cloud::Vantage::new(site).post(
        &seat,
        &json!({
            "enabled": false,
            "previews_enabled": true
        }),
    )?;
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

fn reachable(url: &str, site: &Site) -> Result<(), String> {
    let mut why = String::new();
    for turn in 0..site.turns.max(1) {
        why = match answered(url) {
            Ok(true) => {
                println!("  200 {url}");
                return Ok(());
            }
            Ok(false) => format!("{url} did not answer 200"),
            Err(error) => error,
        };
        println!("  retry {url} ({why})");
        if turn + 1 < site.turns && site.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(site.delay));
        }
    }
    Err(format!("{url} never answered: {why}"))
}

fn answered(url: &str) -> Result<bool, String> {
    let code = super::process::text(
        "curl",
        &[
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            url,
        ],
        Path::new("."),
    )?;
    Ok(code == "200")
}
