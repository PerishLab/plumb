use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error, Limits, Runner, Streams};
use std::process::{Command, Output};
use std::time::Duration;

pub(crate) const OUTPUT: u64 = 65536;
pub(crate) const LIMIT: Duration = Duration::from_secs(10);

pub(crate) fn capture(command: Command, limit: Duration) -> Result<Output, String> {
    let worker = std::thread::Builder::new()
        .name("probe".into())
        .spawn(move || execute(command, limit))
        .map_err(|error| format!("cannot start probe worker: {error}"))?;
    worker
        .join()
        .map_err(|_| "probe worker interrupted; cleanup unconfirmed".to_string())?
}

fn execute(command: Command, limit: Duration) -> Result<Output, String> {
    let runner = Runner::new(Limits {
        time: limit,
        output: OUTPUT,
    })
    .map_err(|error| format!("cannot configure probe: {error}"))?;
    let completed = runner
        .stream(command, Streams::stdout(Vec::new()), Cancel::new())
        .map_err(|error| failure(error, limit))?;
    Ok(Output {
        status: completed.status,
        stdout: completed.streams.stdout.unwrap_or_default(),
        stderr: Vec::new(),
    })
}

fn failure<T>(error: Error<T>, limit: Duration) -> String {
    let Error::Operation(failed) = error else {
        return format!("probe execution interrupted; cleanup unconfirmed: {error}");
    };
    let message = match failed.cause {
        Cause::Time => format!("exceeded {} limit", span(limit)),
        Cause::Output => "stdout exceeds 65536 bytes".to_string(),
        Cause::Cancelled => "probe cancelled".to_string(),
        Cause::Io(error) => format!("cannot execute probe: {error}"),
    };
    if let Some(error) = failed.cleanup {
        return format!(
            "{message}; cleanup unconfirmed for child {:?}: {error}",
            failed.child.as_ref().and_then(|child| child.id())
        );
    }
    message
}

fn span(limit: Duration) -> String {
    if limit.subsec_nanos() == 0 {
        format!("{} second", limit.as_secs())
    } else {
        format!("{} millisecond", limit.as_millis())
    }
}
