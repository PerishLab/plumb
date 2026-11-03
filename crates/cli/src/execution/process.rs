use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

pub(crate) fn run(
    root: &Path,
    argv: &[String],
    context: Option<&plumb::config::Execution>,
) -> Result<(), String> {
    let _held = crate::execution::signal::Hold::new()?;
    let (program, args) = argv.split_first().ok_or("guard action has no command")?;
    let managed =
        context.is_some_and(|context| context.environment.get("CARGO_TARGET_DIR").is_some());
    let cache = (program == "cargo" && !managed)
        .then(|| crate::command::precommit::cargo::Lease::new(root))
        .transpose()?;
    if let Some(context) = context {
        crate::execution::inspect(
            if managed {
                "cargo"
            } else {
                crate::execution::family(program)
            },
            root,
            &context.environment,
        )?;
    }
    let compiler = if program == "cargo" || managed {
        crate::cargo::cache(context)?
    } else {
        None
    };
    let command = || {
        let mut command = match context {
            Some(context) => context.command(program)?,
            None => plumb::config::detached(program),
        };
        if let Some(cache) = &cache {
            crate::cargo::configure(&mut command);
            command.env("CARGO_TARGET_DIR", &cache.root);
        }
        if let Some(compiler) = &compiler {
            compiler.apply(&mut command);
        }
        command.current_dir(root);
        unhook(&mut command);
        Ok::<_, String>(command)
    };
    let selected = argv == ["cargo", "test", "--locked"];
    let foreground = std::io::stdin().is_terminal();
    if selected && foreground {
        eprintln!(
            "guard/test: serial Cargo fallback: terminal input requires foreground job control"
        );
    }
    let result = if cfg!(unix) && selected && !foreground {
        crate::execution::suite::run(root, &command)
    } else {
        let mut command = command()?;
        command.args(args).stdout(std::io::stderr());
        ordinary(command, argv)
    };
    drop(compiler);
    result?;
    crate::execution::signal::check()?;
    if let Some(cache) = &cache {
        cache.settle();
    }
    Ok(())
}

fn ordinary(mut command: Command, argv: &[String]) -> Result<(), String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot run {}: {error}", argv.join(" ")))?;
    let status = crate::execution::signal::wait(&mut child, argv)?;
    crate::diagnostic::event(
        "command.exit",
        || serde_json::json!({"code":status.code(),"success":status.success()}),
    );
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {status}", argv.join(" ")))
    }
}

fn unhook(command: &mut Command) {
    let held = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|_| key.to_os_string()))
        .collect::<Vec<_>>();
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(|key| key.starts_with("GIT_")) && !held.contains(&key) {
            command.env_remove(key);
        }
    }
}
