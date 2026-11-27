use super::Command;
use plumb::depot::Rules;
use plumb::depot::v3::{Channel, Kind, Pinned};
use std::path::PathBuf;

pub(crate) mod brand;
pub mod configuration;
pub(crate) mod drift;
pub(crate) mod instance;
pub mod lane;
pub mod lock;
pub mod metadata;
pub mod skill;
pub mod windtun;

pub(crate) const DEPOT: &str = "https://depot.plumb.perish.uk";
const CONFIGURATION: &str = "PLUMB_CONFIGURATION";
const SOURCE: &str = "PLUMB_RULES_SOURCE";

pub fn prepare(command: &Command) -> Result<(), String> {
    if skipped(command) {
        return Ok(());
    }
    load()?;
    plumb::depot::rules()?;
    crate::catalog::prepare()?;
    crate::catalog::set::prepare()?;
    plumb::vocabulary::Dictionary::synced().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn run(command: Command) {
    let observation = crate::observation::start(command.name());
    let prepared = plumb::identity::ready().and_then(|()| {
        prepare(&command).map_err(|error| format!("cannot resolve Plumb rules: {error}"))
    });
    let diagnostic = crate::diagnostic::Run::start(command.diagnostic(), command.name());
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

pub(super) fn bound() -> bool {
    !cfg!(debug_assertions) || plumb::commit!("PLUMB").is_some()
}

pub(super) fn home() -> Result<PathBuf, String> {
    plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot locate Depot cache: no PLUMB_HOME".to_string())
}

pub(super) fn pin() -> Result<Option<String>, String> {
    std::env::var_os(CONFIGURATION)
        .map(|raw| {
            raw.into_string()
                .map_err(|_| format!("{CONFIGURATION} is not valid UTF-8"))
        })
        .transpose()
}

pub(super) fn origin() -> Result<Option<String>, String> {
    if bound() {
        return Ok(None);
    }
    std::env::var_os(SOURCE)
        .map(|raw| {
            raw.into_string()
                .map_err(|_| format!("{SOURCE} is not valid UTF-8"))
        })
        .transpose()
}

fn skipped(command: &Command) -> bool {
    matches!(
        command,
        Command::Instance { .. }
            | Command::Brand { .. }
            | Command::Configuration { .. }
            | Command::Windtun { .. }
            | Command::Metadata { .. }
            | Command::Release { .. }
            | Command::Ship { .. }
            | Command::Skill { .. }
    ) || recovery(command)
}

fn recovery(command: &Command) -> bool {
    matches!(
        command,
        Command::Depot {
            deed: crate::command::depot::Deed::Consign { .. }
        }
    ) && consigned().is_some_and(|kind| matches!(kind.as_str(), "configuration" | "scene"))
}

fn consigned() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--kind=") {
            return Some(value.to_string());
        }
        if arg == "--kind" {
            return args.next();
        }
        if matches!(arg.as_str(), "--version" | "--dir") {
            let _ = args.next();
        }
    }
    None
}

fn load() -> Result<(), String> {
    if plumb::depot::rules().is_ok() {
        return Ok(());
    }
    let pinned = pin()?;
    if bound() {
        return remote(DEPOT, pinned.as_deref());
    }
    let source = origin()?;
    if pinned.is_some() || source.is_some() {
        return remote(source.as_deref().unwrap_or(DEPOT), pinned.as_deref());
    }
    candidate()
}

fn remote(source: &str, raw: Option<&str>) -> Result<(), String> {
    let home = home()?;
    let rules = match raw {
        Some(raw) => {
            let pinned = Pinned::parse(raw, source)?;
            let bundle = pinned.cached(&home, source)?;
            Rules::resolved(&bundle, pinned)?
        }
        None => {
            let selected = pick(plumb::version!("PLUMB"), |name| {
                Channel {
                    source,
                    product: "plumb",
                    channel: name,
                    kind: Kind::Configuration,
                }
                .pointer()
                .map(|pointer| pointer.version)
            })?;
            let discovered = Channel {
                source,
                product: "plumb",
                channel: selected,
                kind: Kind::Configuration,
            }
            .cached(&home)?;
            let pinned = Pinned::pointer(&discovered.pointer)?;
            Rules::resolved(&discovered.bundle, pinned)?
        }
    };
    plumb::depot::install(rules)
}

#[cfg(debug_assertions)]
fn candidate() -> Result<(), String> {
    plumb::depot::carry(crate::catalog::carried::FILES);
    Ok(())
}

#[cfg(not(debug_assertions))]
fn candidate() -> Result<(), String> {
    Err("release Plumb requires Depot configuration".into())
}

const STABLE: &str = "stable";

fn pick<F>(version: &str, mut read: F) -> Result<&'static str, String>
where
    F: FnMut(&str) -> Result<String, String>,
{
    let Some(candidate) = prerelease(version) else {
        return Ok(STABLE);
    };
    let Ok(staged) = read(candidate) else {
        return Ok(STABLE);
    };
    let Ok(standing) = read(STABLE) else {
        return Ok(candidate);
    };
    if current(&staged, &standing) {
        Ok(candidate)
    } else {
        Ok(STABLE)
    }
}

fn prerelease(version: &str) -> Option<&'static str> {
    match crate::command::release::channel(version).ok()?.as_str() {
        "rc" => Some("rc"),
        "beta" => Some("beta"),
        _ => None,
    }
}

fn current(candidate: &str, standing: &str) -> bool {
    let parse = |raw: &str| semver::Version::parse(raw.strip_prefix('v').unwrap_or(raw));
    match (parse(candidate), parse(standing)) {
        (Ok(left), Ok(right)) => left >= right,
        _ => false,
    }
}
