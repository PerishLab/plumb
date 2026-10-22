mod resource;
mod sink;
mod span;

use serde_json::{Value, json};
use sink::Sink;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) use span::{Span, measure};

static ACTIVE: Mutex<Option<Arc<Mutex<Sink>>>> = Mutex::new(None);

pub(crate) struct Run(Arc<Mutex<Sink>>);

impl Run {
    pub(crate) fn start(path: Option<&Path>, operation: &str) -> Option<Self> {
        let path = path?;
        let sink = match Sink::new(path) {
            Ok(sink) => Arc::new(Mutex::new(sink)),
            Err(_) => {
                eprintln!("plumb diagnostic: cannot create new diagnostic file");
                return None;
            }
        };
        *ACTIVE.lock().unwrap_or_else(|error| error.into_inner()) = Some(Arc::clone(&sink));
        event("start", || {
            json!({
                "operation": operation,
                "plumb": plumb::version!("PLUMB"),
                "depot": plumb::depot::rules().ok().map(|rules| rules.mark().to_owned()),
                "platform": plumb::config::platform(),
                "run": number("GITHUB_RUN_ID"),
                "attempt": number("GITHUB_RUN_ATTEMPT"),
                "process": std::process::id(),
                "locus":crate::observation::view().and_then(|(_,context)|context.read(&locus::Role::trace()).map(|key|key.text().to_owned())),
                "resources": resource::read(),
                "suites": {"status":"unavailable", "reason":"inherited output; existing CI logs retain suite results"},
                "critical_path": null
            })
        });
        Some(Self(sink))
    }

    pub(crate) fn finish(self, code: i32) {
        *ACTIVE.lock().unwrap_or_else(|error| error.into_inner()) = None;
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .finish(code);
    }
}

pub(crate) fn event(kind: &'static str, data: impl FnOnce() -> Value) {
    if let Some(sink) = active() {
        sink.lock()
            .unwrap_or_else(|error| error.into_inner())
            .write(kind, data(), false);
    }
}

fn active() -> Option<Arc<Mutex<Sink>>> {
    ACTIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn number(key: &str) -> Option<u64> {
    std::env::var(key).ok()?.parse().ok()
}
