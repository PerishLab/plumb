use super::artifact::Slot;
use plumb::config::Execution;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) struct Client {
    execution: Execution,
    headers: tempfile::NamedTempFile,
    started: Instant,
    budget: Duration,
}

struct Request<'a> {
    url: &'a str,
    target: &'a Path,
    authenticated: bool,
    limits: (u64, u64),
    headers: Option<&'a Path>,
}

impl Client {
    pub fn new(root: &Path, budget: Duration) -> Result<Option<Self>, String> {
        let token = std::env::var("GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .ok();
        let Some(token) = token.filter(|token| !token.is_empty()) else {
            return Ok(None);
        };
        if !token.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
            return Err("GitHub cache read credential is malformed".into());
        }
        let mut headers =
            tempfile::NamedTempFile::new().map_err(|_| "cannot isolate cache read credential")?;
        write!(headers, "Authorization: Bearer {token}\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\n").map_err(|_| "cannot isolate cache read credential")?;
        let environment = plumb::config::contract("probe")?.capture(std::env::vars_os())?;
        let execution = Execution::new(environment, &["curl".into()], root)?;
        Ok(Some(Self {
            execution,
            headers,
            started: Instant::now(),
            budget,
        }))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T, String> {
        let timeout = self.leftover()?.as_secs().clamp(1, 10);
        let file = tempfile::NamedTempFile::new().map_err(|_| "cannot stage cache metadata")?;
        let status = self.get(Request {
            url: &format!("https://api.github.com/{endpoint}"),
            target: file.path(),
            authenticated: true,
            limits: (65536, timeout),
            headers: None,
        })?;
        if status != "200" {
            return Err(format!("GitHub cache metadata returned HTTP {status}"));
        }
        let bytes = std::fs::read(file.path()).map_err(|_| "cannot read cache metadata")?;
        if bytes.len() > 65536 {
            return Err("cache metadata exceeded 64 KiB".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "GitHub cache metadata is invalid JSON".into())
    }

    pub fn download(&self, endpoint: &str, target: &Path, maximum: u64) -> Result<(), String> {
        let headers =
            tempfile::NamedTempFile::new().map_err(|_| "cannot stage artifact redirect")?;
        let body = tempfile::NamedTempFile::new().map_err(|_| "cannot stage artifact redirect")?;
        let redirect = self.leftover()?.as_secs().clamp(1, 10);
        let status = self.get(Request {
            url: &format!("https://api.github.com/{endpoint}"),
            target: body.path(),
            authenticated: true,
            limits: (65536, redirect),
            headers: Some(headers.path()),
        })?;
        if status != "302" {
            return Err(format!("GitHub cache download returned HTTP {status}"));
        }
        let text =
            std::fs::read_to_string(headers.path()).map_err(|_| "cannot read artifact redirect")?;
        let locations = text
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(name, _)| name.eq_ignore_ascii_case("location"))
            .map(|(_, value)| value.trim())
            .collect::<Vec<_>>();
        if locations.len() != 1 || !super::artifact::location(locations[0]) {
            return Err("GitHub cache artifact redirect is invalid".into());
        }
        let timeout = self.leftover()?.as_secs().max(1);
        let status = self.get(Request {
            url: locations[0],
            target,
            authenticated: false,
            limits: (maximum, timeout),
            headers: None,
        })?;
        if status != "200" {
            return Err(format!("cache artifact storage returned HTTP {status}"));
        }
        Ok(())
    }

    fn leftover(&self) -> Result<Duration, String> {
        self.budget
            .checked_sub(self.started.elapsed())
            .filter(|time| !time.is_zero())
            .ok_or_else(|| {
                format!(
                    "cache restore exceeded its {}s budget",
                    self.budget.as_secs()
                )
            })
    }

    fn get(&self, request: Request<'_>) -> Result<String, String> {
        let Request {
            url,
            target,
            authenticated,
            limits,
            headers,
        } = request;
        let mut command = self.execution.command("curl")?;
        command.args([
            "--disable",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--noproxy",
            "*",
            "--proxy",
            "",
            "--connect-timeout",
            "3",
            "--max-redirs",
            "0",
            "--write-out",
            "%{http_code}",
        ]);
        command
            .args([
                "--max-time",
                &limits.1.to_string(),
                "--max-filesize",
                &limits.0.to_string(),
                "--output",
            ])
            .arg(target);
        if authenticated {
            command
                .arg("--header")
                .arg(format!("@{}", self.headers.path().display()));
        }
        if let Some(headers) = headers {
            command.arg("--dump-header").arg(headers);
        }
        command.arg(url);
        let output = command
            .output()
            .map_err(|_| "cannot run isolated cache read transport")?;
        if !output.status.success() {
            return Err(self
                .leftover()
                .err()
                .unwrap_or_else(|| "cache read transport failed or exceeded its bound".into()));
        }
        let status = std::str::from_utf8(&output.stdout)
            .map_err(|_| "cache HTTP status is invalid")?
            .trim();
        if status.len() != 3 || !status.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("cache HTTP status is invalid".into());
        }
        Ok(status.into())
    }
}

#[derive(Default)]
pub(super) struct Scan {
    pub pages: u64,
    pub scanned: u64,
    pub own: u64,
    pub other: u64,
    pub foreign: u64,
    pub refused: u64,
    pub outside: u64,
}

impl Scan {
    pub fn note(&mut self, name: &str, expired: bool, slot: Option<&Slot>, family: &str) {
        self.scanned += 1;
        match slot {
            Some(slot) if slot.exact => self.own += 1,
            Some(_) => self.other += 1,
            None if name.starts_with(super::FAMILY) && !expired && !name.starts_with(family) => {
                self.foreign += 1;
            }
            None => {}
        }
    }

    pub fn report(&self) -> String {
        if self.own + self.other + self.foreign == 0 {
            return format!(
                "no Guard material artifact among {} artifacts on {} pages",
                self.scanned, self.pages
            );
        }
        if self.own + self.other == 0 {
            return format!(
                "{} Guard material artifacts exist, all for another Guard environment (repository, image or platform changed); {} artifacts on {} pages",
                self.foreign, self.scanned, self.pages
            );
        }
        format!(
            "{} artifacts of this Guard environment ({} from this producer, {} from another): {} from runs not accepted, {} outside the default-branch tree window; {} for other environments; {} artifacts on {} pages",
            self.own + self.other,
            self.own,
            self.other,
            self.refused,
            self.outside,
            self.foreign,
            self.scanned,
            self.pages
        )
    }

    pub fn finish(&self, found: bool) {
        crate::diagnostic::event("material.lookup.scan", || {
            serde_json::json!({
                "found":found,"pages":self.pages,"scanned":self.scanned,"own":self.own,"other":self.other,
                "foreign":self.foreign,"refused":self.refused,"outside":self.outside
            })
        });
        if !found {
            eprintln!("guard material: no accepted artifact: {}", self.report());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reasons() {
        use super::{Scan, Slot};
        let family = format!("guard-material-v1-{}-", "a".repeat(64));
        let other = format!("guard-material-v1-{}-{}-1", "b".repeat(64), "c".repeat(64));
        let mut scan = Scan {
            pages: 1,
            ..Scan::default()
        };
        scan.note("guard-runtime", false, None, &family);
        assert!(
            scan.report()
                .starts_with("no Guard material artifact among 1 artifacts")
        );
        scan.note(&other, false, None, &family);
        assert!(scan.report().contains("all for another Guard environment"));
        scan.note(&other, true, None, &family);
        assert_eq!(scan.foreign, 1);
        let slot = Slot {
            attempt: 2,
            exact: false,
        };
        scan.note(
            &format!("{family}{}-2", "d".repeat(64)),
            false,
            Some(&slot),
            &family,
        );
        scan.refused += 1;
        let report = scan.report();
        assert!(
            report.starts_with("1 artifacts of this Guard environment (0 from this producer, 1 from another): 1 from runs not accepted"),
            "{report}"
        );
        assert!(report.contains("1 for other environments"), "{report}");
    }
}
