use std::io::{Read, Seek, SeekFrom};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub(crate) fn capture(command: &mut Command) -> Result<Output, String> {
    let mut file =
        tempfile::tempfile().map_err(|error| format!("cannot reserve probe output: {error}"))?;
    command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(file.try_clone().map_err(|error| error.to_string())?);
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start: {error}"))?;
    let started = Instant::now();
    let status = loop {
        let size = file.metadata().map(|held| held.len());
        let refusal = match size {
            Err(error) => Some(format!("cannot inspect output: {error}")),
            Ok(size) if size > 65536 => Some("stdout exceeds 65536 bytes".to_string()),
            _ if started.elapsed() >= Duration::from_secs(5) => {
                Some("exceeded 5 second limit".to_string())
            }
            _ => None,
        };
        if let Some(error) = refusal {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("cannot wait: {error}"));
            }
        }
    };
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut stdout = Vec::new();
    file.take(65537)
        .read_to_end(&mut stdout)
        .map_err(|error| error.to_string())?;
    if stdout.len() > 65536 {
        return Err("stdout exceeds 65536 bytes".into());
    }
    Ok(Output {
        status,
        stdout,
        stderr: Vec::new(),
    })
}
