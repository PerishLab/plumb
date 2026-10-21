use plumb::config::Execution;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) struct Client {
    execution: Execution,
    headers: tempfile::NamedTempFile,
    started: Instant,
}

struct Request<'a> {
    url: &'a str,
    target: &'a Path,
    authenticated: bool,
    limits: (u64, u64),
    headers: Option<&'a Path>,
}

impl Client {
    pub fn new(root: &Path) -> Result<Option<Self>, String> {
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
        }))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T, String> {
        let remaining = Duration::from_secs(30)
            .checked_sub(self.started.elapsed())
            .ok_or("cache heat metadata lookup exceeded 30 seconds")?;
        let timeout = remaining.as_secs().clamp(1, 10);
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
        let status = self.get(Request {
            url: &format!("https://api.github.com/{endpoint}"),
            target: body.path(),
            authenticated: true,
            limits: (65536, 10),
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
        if locations.len() != 1 || !location(locations[0]) {
            return Err("GitHub cache artifact redirect is invalid".into());
        }
        let status = self.get(Request {
            url: locations[0],
            target,
            authenticated: false,
            limits: (maximum, 120),
            headers: None,
        })?;
        if status != "200" {
            return Err(format!("cache artifact storage returned HTTP {status}"));
        }
        Ok(())
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
            return Err("cache read transport failed or exceeded its bound".into());
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

fn location(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or_default();
    if url.len() > 8192 {
        return false;
    }
    !host.is_empty() && !host.contains('@') && !host.contains('\\')
}

#[cfg(test)]
mod tests {
    #[test]
    fn redirect() {
        assert!(super::location(
            "https://storage.example/artifact?signature=value"
        ));
        for url in [
            "http://storage.example/a",
            "https:///missing",
            "https://token@storage.example/a",
            "https://storage.example\\@foreign/a",
        ] {
            assert!(!super::location(url), "{url}");
        }
    }
}
