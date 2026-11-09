use super::Command;

pub(crate) mod brand;
pub mod configuration;
pub mod lane;
pub mod lock;
pub mod metadata;
pub mod skill;
pub mod windtun;

pub fn prepare(command: &Command) -> Result<(), String> {
    if matches!(
        command,
        Command::Brand { .. }
            | Command::Configuration { .. }
            | Command::Windtun { .. }
            | Command::Lock { .. }
            | Command::Metadata { .. }
            | Command::Release { .. }
            | Command::Ship { .. }
    ) {
        return Ok(());
    }
    plumb::depot::rules()?;
    crate::catalog::prepare()?;
    crate::catalog::set::prepare()?;
    plumb::vocabulary::Dictionary::synced().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn run(command: Command) {
    let observation = crate::observation::start(command.name());
    let diagnostic = crate::diagnostic::Run::start(command.diagnostic(), command.name());
    let prepared = plumb::identity::ready().and_then(|()| {
        prepare(&command).map_err(|error| {
            format!("cannot read installed rules: {error}\nrun plumb configuration install")
        })
    });
    let code = match prepared {
        Ok(()) => crate::command::execute(command),
        Err(error) => {
            eprintln!("plumb: {error}");
            1
        }
    };
    let code = crate::execution::signal::received().unwrap_or(code);
    if let Some(observation) = observation {
        observation.finish(code);
    }
    if let Some(diagnostic) = diagnostic {
        diagnostic.finish(code);
    }
    std::process::exit(code);
}
