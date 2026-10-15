use plumb::observation::{Declaration, Run};
use std::path::PathBuf;

pub(crate) use plumb::observation::{fault, view};

pub(crate) fn start(command: &str) -> Option<Run> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))?;
    let declaration = Declaration {
        producer: "plumb",
        file: &home.join("plumb.toml"),
        buffer: home.join("state/locus"),
    };
    Run::start(declaration, command)
}
