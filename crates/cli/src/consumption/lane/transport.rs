use std::io::{Read, Seek, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn capture(mut command: Command, body: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let mut input = tempfile::tempfile().map_err(|_| "cannot prepare lane command input")?;
    if let Some(body) = body {
        input
            .write_all(body)
            .map_err(|_| "cannot prepare lane command input")?;
    }
    input
        .rewind()
        .map_err(|_| "cannot rewind lane command input")?;
    let mut output = tempfile::tempfile().map_err(|_| "cannot capture lane command output")?;
    command
        .stdin(input)
        .stdout(
            output
                .try_clone()
                .map_err(|_| "cannot capture lane command output")?,
        )
        .stderr(Stdio::null());
    let mut child = Running(
        command
            .spawn()
            .map_err(|_| "cannot start bounded lane command")?,
    );
    let start = Instant::now();
    loop {
        let size = output
            .metadata()
            .map_err(|_| "cannot bound lane command output")?
            .len();
        if start.elapsed() >= Duration::from_secs(30) || size > 65536 {
            let _ = child.0.kill();
            let _ = child.0.wait();
            return Err("lane command exceeded its time or output bound".into());
        }
        match child.0.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err("lane command refused or unavailable".into()),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.0.kill();
                let _ = child.0.wait();
                return Err("lane command outcome unverified".into());
            }
        }
    }
    output
        .rewind()
        .map_err(|_| "cannot rewind lane command output")?;
    let mut bytes = Vec::new();
    output
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read lane command output")?;
    if bytes.len() > 65536 {
        return Err("lane command exceeded its output bound".into());
    }
    Ok(bytes)
}

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
