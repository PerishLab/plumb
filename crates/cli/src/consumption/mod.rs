use super::Command;

pub mod configuration;
pub mod skill;

pub fn prepare(command: &Command) -> Result<(), String> {
    if matches!(
        command,
        Command::Configuration { .. } | Command::Release { .. } | Command::Ship { .. }
    ) {
        return Ok(());
    }
    plumb::depot::rules()?;
    crate::catalog::prepare()?;
    crate::catalog::set::prepare()?;
    plumb::vocabulary::Dictionary::synced().map_err(|error| error.to_string())?;
    Ok(())
}
