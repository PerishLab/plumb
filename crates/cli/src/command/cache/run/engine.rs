use super::{Input, owner::Scope, record::Report};
use plumb::config::{Binding, Contract, Execution};
use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error, Limits, Runner, Streams};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const OUTPUT: u64 = 64 * 1024 * 1024;

pub(super) fn context(input: &Input, scope: &Scope) -> Result<Execution, String> {
    let bind = [
        ("CARGO_TARGET_DIR", "target"),
        ("PLUMB_HOME", "plumb"),
        ("TMPDIR", "tmp"),
    ]
    .into_iter()
    .map(|(key, value)| {
        (
            key.to_string(),
            Binding::Value(scope.directory.join(value).display().to_string()),
        )
    })
    .collect::<BTreeMap<_, _>>();
    for name in ["target", "plumb", "tmp"] {
        std::fs::create_dir_all(scope.directory.join(name)).map_err(|error| error.to_string())?;
    }
    let contract = Contract {
        inherit: [
            "PATH",
            "HOME",
            "USER",
            "LOGNAME",
            "LANG",
            "LC_ALL",
            "TZ",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "CARGO_BUILD_JOBS",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        managed: Vec::new(),
        reject: Vec::new(),
        bind,
    };
    let values = std::env::vars_os().filter(|(key, _)| {
        key.to_str()
            .is_none_or(|key| !contract.bind.contains_key(key))
    });
    let environment = contract.capture(values)?;
    Execution::new(environment, &input.arguments[..1], &input.cwd)
}

pub(super) fn execute(input: &Input, context: &Execution, output: &Path) -> Result<Report, String> {
    let mut command = context.command(&input.arguments[0])?;
    command.args(&input.arguments[1..]);
    let stdout =
        std::fs::File::create_new(output.join("stdout.log")).map_err(|error| error.to_string())?;
    let stderr =
        std::fs::File::create_new(output.join("stderr.log")).map_err(|error| error.to_string())?;
    let streams = Streams {
        input: None::<Cursor<Vec<u8>>>,
        stdout: Some(tokio::fs::File::from_std(stdout)),
        stderr: Some(tokio::fs::File::from_std(stderr)),
    };
    let runner = Runner::group(Limits {
        time: Duration::from_secs(input.timeout),
        output: OUTPUT,
    })
    .map_err(|error| error.to_string())?;
    let cancel = Cancel::new();
    let done = Arc::new(AtomicBool::new(false));
    let watcher = watch(done.clone(), cancel.clone());
    let completed = runner.stream(command, streams, cancel);
    done.store(true, Ordering::Release);
    watcher
        .join()
        .map_err(|_| "signal observer failed".to_string())?;
    match completed {
        Ok(completed) => Ok(report(Some(completed.status), "exit", None)),
        Err(Error::Operation(failed)) => {
            let cause = match &failed.cause {
                Cause::Time => "timeout",
                Cause::Cancelled => "cancelled",
                Cause::Output => "output",
                Cause::Io(_) => "io",
            };
            let error = match (&failed.cause, &failed.cleanup) {
                (_, Some(error)) | (Cause::Io(error), None) => Some(error.to_string()),
                _ => None,
            };
            let mut value = report(failed.status, cause, error);
            value.process = failed.cleanup.is_none() && failed.child.is_none();
            Ok(value)
        }
        Err(error) => {
            let mut value = Report::failed(error.to_string());
            value.process = false;
            Ok(value)
        }
    }
}

fn watch(done: Arc<AtomicBool>, cancel: Cancel) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !done.load(Ordering::Acquire) {
            if crate::execution::signal::received().is_some() {
                cancel.cancel();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })
}

fn report(status: Option<ExitStatus>, cause: &str, error: Option<String>) -> Report {
    let code = status.as_ref().and_then(ExitStatus::code);
    let signal = status.as_ref().and_then(ExitStatusExt::signal);
    let exit = match cause {
        "timeout" => 124,
        "cancelled" => crate::execution::signal::received().unwrap_or(130),
        "exit" => code
            .or_else(|| signal.map(|signal| 128 + signal))
            .unwrap_or(125),
        _ => 125,
    };
    Report {
        schema: "plumb.cache-run/v1",
        code,
        signal,
        cause: cause.into(),
        process: error.is_none(),
        directory: false,
        cleanup: false,
        exit,
        error,
    }
}
