use super::{Sink, active};
use std::sync::{Arc, Mutex};

pub(crate) struct Span {
    sink: Option<Arc<Mutex<Sink>>>,
    id: u64,
    started: u64,
    status: &'static str,
}

impl Span {
    pub(crate) fn new(operation: &'static str) -> Self {
        let sink = active();
        let (id, started) = sink.as_ref().map_or((0, 0), |sink| {
            sink.lock()
                .unwrap_or_else(|error| error.into_inner())
                .enter(operation)
        });
        Self {
            sink,
            id,
            started,
            status: "incomplete",
        }
    }

    pub(crate) fn complete(&mut self, success: bool) {
        self.status = if success { "success" } else { "failed" };
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.lock()
                .unwrap_or_else(|error| error.into_inner())
                .leave(self.id, self.started, self.status);
        }
    }
}

pub(crate) fn measure<T>(
    operation: &'static str,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let mut span = Span::new(operation);
    let result = work();
    span.complete(result.is_ok());
    result
}
