use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn run(root: &Path, family: &str, argv: &[String]) -> Result<Vec<u8>, String> {
    let (program, args) = argv.split_first().ok_or("package command is empty")?;
    let environment = crate::config::environment(&crate::config::contract(family)?)?;
    let execution =
        crate::config::Execution::new(environment, std::slice::from_ref(program), root)?;
    let mut command = execution.command(program)?;
    command.args(args).current_dir(root);
    capture(command, program)
}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let argv = std::iter::once("git".to_string())
        .chain(args.iter().map(|arg| arg.to_string()))
        .collect::<Vec<_>>();
    let bytes = run(root, "probe", &argv)?;
    String::from_utf8(bytes).map_err(|error| format!("git output is not UTF-8: {error}"))
}

fn capture(mut command: Command, program: &str) -> Result<Vec<u8>, String> {
    let secrets = secrets(&command);
    let mut out = tempfile::tempfile().map_err(|error| error.to_string())?;
    let mut err = tempfile::tempfile().map_err(|error| error.to_string())?;
    command
        .stdin(Stdio::null())
        .stdout(out.try_clone().map_err(|error| error.to_string())?)
        .stderr(err.try_clone().map_err(|error| error.to_string())?);
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot run package command: {error}"))?;
    let started = Instant::now();
    let status = loop {
        if started.elapsed() > Duration::from_secs(120)
            || out.metadata().map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024
            || err.metadata().map_err(|error| error.to_string())?.len() > 65536
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("package command exceeded its time or output bound".into());
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    use std::io::{Read, Seek};
    out.rewind().map_err(|error| error.to_string())?;
    err.rewind().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    out.take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 8 * 1024 * 1024
        || err.metadata().map_err(|error| error.to_string())?.len() > 65536
    {
        return Err("package command exceeded its output bound".into());
    }
    if !status.success() {
        let mut errors = Vec::new();
        err.take(65536)
            .read_to_end(&mut errors)
            .map_err(|error| error.to_string())?;
        let name = Path::new(program).file_name().unwrap_or_default();
        return Err(format!(
            "package command {:?} failed ({status})\nstdout: {}\nstderr: {}",
            name,
            message(&bytes, &secrets),
            message(&errors, &secrets)
        ));
    }
    Ok(bytes)
}

fn secrets(command: &Command) -> Vec<String> {
    let mut values = command
        .get_envs()
        .filter(|(key, _)| {
            let key = key.to_string_lossy().to_ascii_uppercase();
            ["TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "AUTH"]
                .iter()
                .any(|part| key.contains(part))
        })
        .filter_map(|(_, value)| value)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values
}

fn message(bytes: &[u8], secrets: &[String]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    for secret in secrets {
        text = text.replace(secret, "[redacted]");
    }
    let text = text
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect::<String>();
    let text = text.trim();
    if text.is_empty() {
        return "<no output>".into();
    }
    if text.chars().count() <= 4096 {
        return text.into();
    }
    let first = text.chars().take(2048).collect::<String>();
    let last = text.chars().rev().take(2048).collect::<Vec<_>>();
    let last = last.into_iter().rev().collect::<String>();
    format!("{first}\n[output truncated]\n{last}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(mode: &str) -> String {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .env_clear()
            .args([
                "--exact",
                "packages::process::tests::fixture",
                "--nocapture",
            ])
            .env("PLUMB_PACKAGE_DIAGNOSTIC", mode)
            .env("WHARF_PACKAGES_TOKEN", "fixture-private-token");
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        capture(command, "pnpm").unwrap_err()
    }

    #[test]
    fn fixture() {
        let Ok(mode) = std::env::var("PLUMB_PACKAGE_DIAGNOSTIC") else {
            return;
        };
        match mode.as_str() {
            "stdout" => println!("ERR_PNPM_FETCH_401 registry authentication failed"),
            "mixed" => {
                eprintln!("WARN configuration replacement failed");
                println!("ERR_PNPM_FETCH_401 registry authentication failed");
            }
            "secret" => {
                let secret = std::env::var("WHARF_PACKAGES_TOKEN").unwrap();
                println!("Authorization: Bearer {secret}");
                eprintln!("registry rejected {secret}");
            }
            "large" => println!("{}final actionable cause", "界".repeat(8000)),
            "empty" => {}
            _ => panic!("unknown fixture"),
        }
        std::process::exit(7);
    }

    #[test]
    fn stdout() {
        let message = failed("stdout");
        assert!(message.contains("pnpm"), "{message}");
        assert!(
            message.contains("exit code: 7") || message.contains("exit status: 7"),
            "{message}"
        );
        assert!(message.contains("ERR_PNPM_FETCH_401"), "{message}");
    }

    #[test]
    fn mixed() {
        let message = failed("mixed");
        assert!(message.contains("ERR_PNPM_FETCH_401"), "{message}");
        assert!(
            message.contains("WARN configuration replacement failed"),
            "{message}"
        );
    }

    #[test]
    fn credential() {
        let message = failed("secret");
        assert!(!message.contains("fixture-private-token"), "{message}");
        assert!(message.contains("[redacted]"), "{message}");
    }

    #[test]
    fn empty() {
        let message = failed("empty");
        assert!(message.contains("pnpm"), "{message}");
        assert!(message.contains("stderr: <no output>"), "{message}");
    }

    #[test]
    fn bounded() {
        let message = failed("large");
        assert!(message.len() < 15000, "{}", message.len());
        assert!(message.contains("[output truncated]"));
        assert!(message.contains("final actionable cause"));
    }

    #[test]
    fn overlap() {
        let mut command = Command::new("fixture");
        command.env("ONE_TOKEN", "private");
        command.env("TWO_SECRET", "private-long-value");
        assert_eq!(
            message(b"private-long-value private", &secrets(&command)),
            "[redacted] [redacted]"
        );
    }
}
