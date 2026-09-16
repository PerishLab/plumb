use super::Command;

pub mod configuration;
pub mod skill;

pub fn prepare(command: &Command) -> Result<(), String> {
    match command {
        Command::Guard {
            target,
            candidate:
                crate::command::depot::candidate::Selection {
                    marker: Some(marker),
                    generation: Some(generation),
                },
            ..
        } => {
            crate::command::depot::candidate::select(
                std::path::Path::new(&target.root),
                marker,
                generation,
            )?;
        }
        Command::Guard { target, .. } | Command::Land { target, .. } => {
            crate::command::depot::candidate::restore(std::path::Path::new(&target.root))?;
        }
        _ => (),
    }
    if matches!(
        command,
        Command::Authority { .. }
            | Command::Depot { .. }
            | Command::Configuration { .. }
            | Command::Release { .. }
            | Command::Ship { .. }
            | Command::Version { .. }
    ) {
        return Ok(());
    }
    plumb::depot::rules()?;
    crate::catalog::prepare()?;
    crate::catalog::set::prepare()?;
    plumb::vocabulary::Dictionary::synced().map_err(|error| error.to_string())?;
    Ok(())
}
