use locus::{Candidate, Config, Context, Engine, Key, Policy, Role};
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const ENDPOINT: &str = "http://127.0.0.1:43308";

#[derive(Default, Deserialize)]
struct File {
    #[serde(default)]
    locus: Settings,
}

#[derive(Default, Deserialize)]
struct Settings {
    #[serde(default)]
    enabled: bool,
    endpoint: Option<String>,
}

struct Seat {
    engine: Engine,
    context: Context,
}

static SEAT: OnceLock<Seat> = OnceLock::new();
static FAULT: OnceLock<&'static str> = OnceLock::new();

pub struct Declaration<'a> {
    pub producer: &'static str,
    pub file: &'a Path,
    pub buffer: PathBuf,
}

pub struct Run {
    producer: &'static str,
    context: Context,
}

impl Run {
    pub fn start(declaration: Declaration<'_>, command: &str) -> Option<Self> {
        let producer = declaration.producer;
        match load(declaration, command) {
            Ok(run) => run,
            Err(error) => {
                eprintln!("{producer} locus: {error}");
                None
            }
        }
    }

    pub fn finish(self, code: i32) {
        let Some((engine, _)) = view() else {
            return;
        };
        let mut candidate = Candidate::event(json!({"event": "cli.finish", "code": code}));
        if code != 0
            && let Some(fault) = FAULT.get()
            && let (Ok(role), Ok(key)) = (
                Role::new(format!("{}.fault", self.producer)),
                Key::new(*fault),
            )
        {
            candidate = candidate.explicit(role, key);
        }
        let _ = engine.append(&self.context, candidate);
    }
}

pub fn view() -> Option<(&'static Engine, &'static Context)> {
    SEAT.get().map(|seat| (&seat.engine, &seat.context))
}

pub fn fault(code: &'static str) {
    let _ = FAULT.set(code);
}

fn load(declaration: Declaration<'_>, command: &str) -> Result<Option<Run>, String> {
    let file: File = match crate::config::load(declaration.file) {
        Ok(file) => file,
        Err(crate::config::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(None);
        }
        Err(crate::config::Error::Parse { path, .. }) => {
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
    let (seat, cycle) =
        build(&declaration, command, endpoint).map_err(|error| error.to_string())?;
    SEAT.set(seat)
        .map_err(|_| "observation already initialized".to_string())?;
    Ok(Some(Run {
        producer: declaration.producer,
        context: cycle,
    }))
}

fn build(
    declaration: &Declaration<'_>,
    command: &str,
    endpoint: String,
) -> Result<(Seat, Context), locus::Error> {
    let producer = declaration.producer;
    let engine = Engine::bootstrap(Config::new(Policy::default().producer(producer).reporter(
        locus::reporter::Spec::api(endpoint, declaration.buffer.clone()),
    )))?;
    let candidate = Candidate::context().ensure(Role::trace()).explicit(
        Role::new(format!("{producer}.command"))?,
        Key::new(command)?,
    );
    let context = engine.append(&Context::empty(), candidate)?.context();
    let cycle = engine
        .append(
            &context,
            Candidate::event(json!({"event": "cli.start"})).ensure(Role::span()),
        )?
        .context();
    Ok((Seat { engine, context }, cycle))
}
