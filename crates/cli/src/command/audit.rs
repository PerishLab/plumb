use locus::collector;
use locus::generator;
use locus::reporter;
use locus::{Candidate, Config, Context, Engine, Key, Policy, Role};
use plumb::config::Cascade as _;
use plumb::rig::Locus;
use serde_json::json;

const TARGET: &str = "target";
const THREAD: &str = "codex.thread";
const LIMIT: usize = 512;

fn inherit() -> Result<Locus, plumb::config::Error> {
    let seen = <Locus as plumb::config::Cascade>::env("PLUMB_LOCUS")?;
    Ok(Locus::default().merge(seen))
}

pub(crate) struct Run {
    engine: Engine,
    context: Context,
    command: &'static str,
}

impl Run {
    pub(crate) fn start(command: &'static str) -> Option<Self> {
        let settings = match inherit() {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("plumb audit config: {error}");
                return None;
            }
        };
        if !settings.enabled {
            return None;
        }
        let (engine, explicit, target) = bootstrap(settings)?;
        let trace = Role::trace();
        let mut candidate = Candidate::event(json!({
            "event": "cli.start",
            "command": command,
        }))
        .collect(trace.clone(), THREAD)
        .ensure(trace.clone())
        .ensure(Role::span());
        if let Some(role) = target {
            candidate = candidate.collect(role, TARGET);
        }
        if let Some(key) = explicit {
            candidate = candidate.explicit(trace, key);
        }
        let accepted = locus::record!(&engine, &Context::empty(), candidate).ok()?;
        Some(Self {
            engine,
            context: accepted.context(),
            command,
        })
    }

    pub(crate) fn finish(self, code: i32) {
        let candidate = Candidate::event(json!({
            "event": "cli.finish",
            "command": self.command,
            "code": code,
        }))
        .ensure(Role::trace())
        .ensure(Role::span());
        let _ = locus::record!(&self.engine, &self.context, candidate);
    }
}

fn bootstrap(settings: Locus) -> Option<(Engine, Option<Key>, Option<Role>)> {
    let mut policy = Policy::default()
        .collector(
            THREAD,
            collector::Spec::environment("CODEX_THREAD_ID", LIMIT),
        )
        .reporter(reporter::Spec::file(settings.report.file));
    if !settings.trace.file.as_os_str().is_empty() {
        policy = policy.generator(Role::trace(), generator::Spec::shared(settings.trace.file));
    }
    let explicit = (!settings.trace.id.is_empty())
        .then(|| Key::new(settings.trace.id))
        .transpose()
        .ok()?;
    let target = (!settings.target.collectors.is_empty()).then_some(settings.target.collectors);
    let role = target
        .as_ref()
        .map(|_| Role::new("plumb.target"))
        .transpose()
        .ok()?;
    for held in target.iter().flat_map(|value| value.split(',')) {
        policy = policy.collector(TARGET, spec(held.trim()));
    }
    Engine::bootstrap(Config::new(policy))
        .ok()
        .map(|engine| (engine, explicit, role))
}

fn spec(value: &str) -> collector::Spec {
    let Some((name, selector)) = value.split_once(':') else {
        return collector::Spec::new(value, json!({}));
    };
    match name {
        "environment" => collector::Spec::environment(selector, LIMIT),
        "argv" => match selector.parse() {
            Ok(index) => collector::Spec::argument(index, LIMIT),
            Err(_) => collector::Spec::new("argv", json!({"index": selector, "limit": LIMIT})),
        },
        "process" => collector::Spec::process(selector, LIMIT),
        _ => collector::Spec::new(name, json!({})),
    }
}
