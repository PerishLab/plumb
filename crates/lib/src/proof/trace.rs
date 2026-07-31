use crate::context::Context;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub use plumb_macro::span;

static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Record {
    pub id: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub tags: Vec<(String, String)>,
    pub opened: SystemTime,
    pub closed: SystemTime,
}

impl Record {
    pub fn says(&self, key: &str, value: &str) -> bool {
        self.tags
            .iter()
            .any(|(seen, held)| seen == key && held == value)
    }
}

pub trait Engine: Send + Sync {
    fn accept(&self, record: Record);
}

#[derive(Clone)]
pub struct Sink {
    engine: Arc<dyn Engine>,
}

impl Sink {
    pub fn accept(&self, record: Record) {
        self.engine.accept(record);
    }
}

impl<E: Engine + 'static> From<Arc<E>> for Sink {
    fn from(engine: Arc<E>) -> Sink {
        Sink { engine }
    }
}

#[derive(Default)]
pub struct Ledger {
    records: Mutex<Vec<Record>>,
}

impl Ledger {
    pub fn query(&self, keep: impl Fn(&Record) -> bool) -> Vec<Record> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| keep(record))
            .cloned()
            .collect()
    }
}

impl Engine for Ledger {
    fn accept(&self, record: Record) {
        self.records.lock().unwrap().push(record);
    }
}

pub struct Span {
    id: u64,
    parent: Option<u64>,
    name: String,
    tags: Mutex<Vec<(String, String)>>,
    opened: SystemTime,
    sink: Option<Sink>,
}

impl Span {
    pub fn open(name: &str) -> Span {
        let current = Context::current();
        Span {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            parent: current.get::<Span>().map(|span| span.id),
            name: name.to_string(),
            tags: Mutex::new(Vec::new()),
            opened: SystemTime::now(),
            sink: current.get::<Sink>().map(|held| (*held).clone()),
        }
    }

    pub fn note(key: &str, value: &str) {
        if let Some(span) = Context::current().get::<Span>() {
            span.tags
                .lock()
                .unwrap()
                .push((key.to_string(), value.to_string()));
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.accept(Record {
                id: self.id,
                parent: self.parent,
                name: std::mem::take(&mut self.name),
                tags: std::mem::take(&mut *self.tags.lock().unwrap()),
                opened: self.opened,
                closed: SystemTime::now(),
            });
        }
    }
}
