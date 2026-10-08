use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn run(root: &Path, family: &str, argv: &[String]) -> Result<Vec<u8>, String> {
    let (program, args) = argv.split_first().ok_or("package command is empty")?;
    let environment = crate::config::environment(&crate::config::contract(family)?)?;
    let mut command = crate::config::detached(program);
    environment.apply(&mut command);
    command.args(args).current_dir(root);
    capture(command)
}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let argv = std::iter::once("git".to_string())
        .chain(args.iter().map(|arg| arg.to_string()))
        .collect::<Vec<_>>();
    let bytes = run(root, "probe", &argv)?;
    String::from_utf8(bytes).map_err(|error| format!("git output is not UTF-8: {error}"))
}

fn capture(mut command: Command) -> Result<Vec<u8>, String> {
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
    out.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if !status.success() {
        let mut message = String::new();
        err.take(65536)
            .read_to_string(&mut message)
            .map_err(|error| error.to_string())?;
        return Err(format!(
            "package command failed ({status}): {}",
            message.trim()
        ));
    }
    Ok(bytes)
}
