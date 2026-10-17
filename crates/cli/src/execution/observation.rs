use locus::{Candidate, Context, Engine, Key, Role};
use plumb::observation::{Declaration, Run};
use std::path::PathBuf;

pub(crate) use plumb::observation::{fault, view};

const PRODUCER: &str = "plumb";

pub(crate) type Frame = (&'static Engine, Context);

pub(crate) fn start(command: &str) -> Option<Run> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))?;
    let declaration = Declaration {
        producer: PRODUCER,
        file: &home.join("plumb.toml"),
        buffer: home.join("state/locus"),
    };
    Run::start(declaration, command)
}

pub(crate) fn bind(role: &str, key: &str) -> Option<Frame> {
    let (engine, context) = view()?;
    let bound = Role::new(format!("{PRODUCER}.{role}"))
        .and_then(|role| Ok(Candidate::context().explicit(role, Key::new(key)?)))
        .and_then(|candidate| engine.append(context, candidate))
        .map(|accepted| accepted.context())
        .unwrap_or_else(|_| context.clone());
    Some((engine, bound))
}
