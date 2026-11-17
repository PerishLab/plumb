#[cfg(target_os = "linux")]
mod engine;
#[cfg(target_os = "linux")]
mod owner;
#[cfg(target_os = "linux")]
mod record;

use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct Input {
    #[arg(long, value_name = "NEW_DIRECTORY")]
    output: PathBuf,
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    #[arg(long, default_value_t = 900, value_parser = clap::value_parser!(u64).range(1..=86400))]
    timeout: u64,
    #[arg(long)]
    json: bool,
    #[arg(last = true, required = true, allow_hyphen_values = true)]
    arguments: Vec<String>,
}

pub fn run(input: Input) -> i32 {
    #[cfg(target_os = "linux")]
    {
        native(input)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = input;
        eprintln!("plumb cache run: disposable compilation scopes require Linux");
        1
    }
}

#[cfg(target_os = "linux")]
fn native(input: Input) -> i32 {
    match execute(input) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("plumb cache run: {error}");
            125
        }
    }
}

#[cfg(target_os = "linux")]
fn execute(mut input: Input) -> Result<i32, String> {
    use std::os::unix::fs::PermissionsExt;
    let _held = crate::execution::signal::Hold::new()?;
    crate::execution::signal::watch();
    if std::env::var_os("CARGO_TARGET_DIR").is_some_and(|value| !value.is_empty()) {
        return Err("CARGO_TARGET_DIR is already bound; preserve the existing build scope".into());
    }
    input.cwd = input
        .cwd
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !input.cwd.is_dir() {
        return Err("source root is not a directory".into());
    }
    let output = plumb::config::rebase(
        &input.output,
        &std::env::current_dir().map_err(|error| error.to_string())?,
    );
    std::fs::create_dir(&output)
        .map_err(|error| format!("new output directory required: {error}"))?;
    std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    let scope = owner::Scope::new(&super::bucket::home()?)?;
    let execution = engine::context(&input, &scope).and_then(|context| {
        let invocation = record::Invocation {
            schema: "plumb.cache-run-invocation/v1",
            cwd: &input.cwd,
            arguments: &input.arguments,
            directory: &scope.directory,
            registry: &scope.registry,
            tools: context.tools()?,
            timeout: input.timeout,
        };
        record::write(&output.join("invocation.json"), &invocation)?;
        engine::execute(&input, &context, &output)
    });
    let mut report = execution.unwrap_or_else(record::Report::failed);
    if report.process {
        match scope.remove() {
            Ok(()) => report.directory = true,
            Err(error) => report.error = Some(error),
        }
    }
    report.cleanup = report.process && report.directory;
    if !report.cleanup {
        report.exit = 125;
    }
    record::write(&output.join("result.json"), &report)?;
    if input.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        println!(
            "plumb cache run: exit {}; cleanup {}; evidence {}",
            report.exit,
            report.cleanup,
            output.display()
        );
    }
    Ok(report.exit)
}
