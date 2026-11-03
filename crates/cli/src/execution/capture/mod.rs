mod configuration;
mod context;
mod image;
mod session;

#[cfg(all(test, target_os = "linux"))]
mod tests;

pub(crate) use configuration::inspect;
pub(crate) use context::Context;
pub(crate) use image::Image;
pub(crate) use session::Session;
use std::ffi::OsString;
use std::path::Path;

pub(crate) fn run(root: &Path, arguments: &[OsString]) -> i32 {
    match record(root, arguments) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("plumb runner: {error}");
            1
        }
    }
}

fn record(root: &Path, arguments: &[OsString]) -> Result<(), String> {
    let declaration = session::Declaration::read(root)?;
    let (executable, arguments) = arguments
        .split_first()
        .ok_or("Cargo runner has no executable")?;
    let executable = context::directory(Path::new(executable))?;
    let (index, admission) = declaration
        .programs
        .iter()
        .enumerate()
        .find(|(_, program)| program.tool.path == executable)
        .ok_or("Cargo runner executable was not admitted")?;
    let captured = Context::capture(admission, arguments)?;
    session::write(&root.join(format!("{index}.json")), &captured)
}
