use locus::{Adaptor, Candidate, Config, Context, Engine, Hook, Key, Policy, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock, RwLock};

const ENDPOINT: &str = "http://127.0.0.1:43308";
const PARENT: &str = "parent";

#[derive(Default, Deserialize)]
struct File {
    #[serde(default)]
    locus: Legacy,
}

#[derive(Default, Deserialize)]
struct Legacy {
    #[serde(default)]
    enabled: bool,
    endpoint: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub enabled: bool,
    pub endpoint: Option<String>,
}

struct Seat {
    engine: Engine,
    context: Context,
    fault: OnceLock<&'static str>,
}
static ACTIVE: RwLock<Option<&'static Seat>> = RwLock::new(None);

pub struct Declaration<'a> {
    pub producer: &'static str,
    pub file: &'a Path,
    pub buffer: PathBuf,
}
pub struct Run {
    producer: &'static str,
    seat: &'static Seat,
    cycle: Context,
}

impl Run {
    pub fn start(declaration: Declaration<'_>, command: &str) -> Option<Self> {
        let producer = declaration.producer;
        match Self::open(declaration, command, Arc::new(Adaptor)) {
            Ok(run) => run,
            Err(error) => {
                eprintln!("{producer} locus: {error}");
                None
            }
        }
    }
    pub fn open(
        declaration: Declaration<'_>,
        command: &str,
        hook: Arc<dyn Hook>,
    ) -> Result<Option<Self>, String> {
        load(declaration, command, hook)
    }
    pub fn configured(
        declaration: Declaration<'_>,
        command: &str,
        settings: Settings,
        hook: Arc<dyn Hook>,
    ) -> Result<Option<Self>, String> {
        if !settings.enabled {
            return Ok(None);
        }
        let endpoint = settings.endpoint.unwrap_or_else(|| ENDPOINT.into());
        let (seat, cycle) =
            build(&declaration, command, endpoint, hook).map_err(|error| error.to_string())?;
        let seat: &'static Seat = Box::leak(Box::new(seat));
        *ACTIVE.write().unwrap_or_else(|error| error.into_inner()) = Some(seat);
        Ok(Some(Run {
            producer: declaration.producer,
            seat,
            cycle,
        }))
    }
    pub fn view(&self) -> (&'static Engine, &'static Context) {
        (&self.seat.engine, &self.seat.context)
    }
    pub fn fault(&self, code: &'static str) {
        let _ = self.seat.fault.set(code);
    }

    pub fn finish(self, code: i32) {
        let seat = self.seat;
        let mut active = ACTIVE.write().unwrap_or_else(|error| error.into_inner());
        if active.is_some_and(|current| std::ptr::eq(current, seat)) {
            *active = None;
        }
        drop(active);
        let mut candidate = Candidate::event(json!({"event": "cli.finish", "code": code}));
        if code != 0
            && let Some(fault) = seat.fault.get()
            && let (Ok(role), Ok(key)) = (
                Role::new(format!("{}.fault", self.producer)),
                Key::new(*fault),
            )
        {
            candidate = candidate.explicit(role, key);
        }
        let _ = seat.engine.append(&self.cycle, candidate);
    }
}
pub fn view() -> Option<(&'static Engine, &'static Context)> {
    active().map(|seat| (&seat.engine, &seat.context))
}
pub fn fault(code: &'static str) {
    if let Some(seat) = active() {
        let _ = seat.fault.set(code);
    }
}

pub fn parent(command: &mut Command, frame: &Context) {
    if view().is_none() {
        return;
    }
    if let Some(span) = frame.read(&Role::span()) {
        command.env(locus::collector::PARENT, span.text());
    }
}

fn active() -> Option<&'static Seat> {
    *ACTIVE.read().unwrap_or_else(|error| error.into_inner())
}

fn load(
    declaration: Declaration<'_>,
    command: &str,
    hook: Arc<dyn Hook>,
) -> Result<Option<Run>, String> {
    let file: File = match radicula_config::load(declaration.file) {
        Ok(file) => file,
        Err(radicula_config::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(None);
        }
        Err(radicula_config::Error::Parse { path, .. }) => {
            return Err(format!(
                "invalid observation configuration in {}; check [locus] settings",
                path.display()
            ));
        }
        Err(error) => return Err(error.to_string()),
    };
    if !file.locus.enabled {
        return Ok(None);
    }
    let endpoint = file
        .locus
        .endpoint
        .or_else(|| crate::config::value("LOCUS_API"))
        .unwrap_or_else(|| ENDPOINT.into());
    Run::configured(
        declaration,
        command,
        Settings {
            enabled: true,
            endpoint: Some(endpoint),
        },
        hook,
    )
}

fn build(
    declaration: &Declaration<'_>,
    command: &str,
    endpoint: String,
    hook: Arc<dyn Hook>,
) -> Result<(Seat, Context), locus::Error> {
    let producer = declaration.producer;
    let engine = Engine::bootstrap(
        Config::new(
            Policy::default()
                .producer(producer)
                .collector(PARENT, locus::collector::Spec::parent())
                .reporter(locus::reporter::Spec::api(
                    endpoint,
                    declaration.buffer.clone(),
                )),
        )
        .hook(hook),
    )?;
    let candidate = Candidate::context()
        .ensure(Role::trace())
        .explicit(
            Role::new(format!("{producer}.command"))?,
            Key::new(command)?,
        )
        .collect(Role::parent(), PARENT);
    let context = engine.append(&Context::empty(), candidate)?.context();
    let cycle = engine
        .append(
            &context,
            Candidate::event(json!({"event": "cli.start"})).ensure(Role::span()),
        )?
        .context();
    Ok((
        Seat {
            engine,
            context,
            fault: OnceLock::new(),
        },
        cycle,
    ))
}
