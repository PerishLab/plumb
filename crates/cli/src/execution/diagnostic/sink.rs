use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const LIMIT: usize = 1024 * 1024;
const RESERVE: usize = 4096;
const RECORD: usize = 64 * 1024;

pub(super) struct Sink {
    file: File,
    started: Instant,
    bytes: usize,
    dropped: u64,
    failed: bool,
    incomplete: bool,
    next: u64,
    stack: Vec<u64>,
}

impl Sink {
    pub(super) fn new(path: &Path) -> std::io::Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Ok(Self {
            file: options.open(path)?,
            started: Instant::now(),
            bytes: 0,
            dropped: 0,
            failed: false,
            incomplete: false,
            next: 0,
            stack: Vec::new(),
        })
    }

    pub(super) fn offset(&self) -> u64 {
        self.started
            .elapsed()
            .as_nanos()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    pub(super) fn enter(&mut self, operation: &'static str) -> (u64, u64) {
        self.next += 1;
        let id = self.next;
        let started = self.offset();
        self.write(
            "enter",
            json!({"id":id,"parent":self.stack.last(),"operation":operation,"start_offset_ns":started}),
            false,
        );
        self.stack.push(id);
        (id, started)
    }

    pub(super) fn leave(&mut self, id: u64, started: u64, status: &str) {
        let ended = self.offset();
        self.incomplete |= status == "incomplete";
        self.stack.retain(|held| *held != id);
        self.write(
            "leave",
            json!({"id":id,"duration_ns":ended.saturating_sub(started),"end_offset_ns":ended,"status":status}),
            false,
        );
    }

    pub(super) fn write(&mut self, kind: &str, data: Value, terminal: bool) {
        if self.failed {
            return;
        }
        let value = json!({"schema":"plumb.guard-diagnostic/v1","kind":kind,"offset_ns":self.offset(),"span":self.stack.last(),"data":data});
        let Ok(mut bytes) = serde_json::to_vec(&value) else {
            self.dropped += 1;
            return;
        };
        bytes.push(b'\n');
        let limit = if terminal { LIMIT } else { LIMIT - RESERVE };
        if bytes.len() > RECORD || self.bytes.saturating_add(bytes.len()) > limit {
            self.dropped += 1;
            return;
        }
        self.bytes += bytes.len();
        if self.file.write_all(&bytes).is_err() {
            self.failed = true;
            super::warn("diagnostic write failed; partial evidence only");
        }
    }

    pub(super) fn finish(&mut self, code: i32) {
        self.write("finish", json!({
            "exit_code":code,
            "capture":if self.dropped == 0 && self.stack.is_empty() && !self.incomplete {"complete"} else {"partial"},
            "dropped":self.dropped,
            "open_spans":self.stack.len(),
            "resources":super::resource::read()
        }), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("diagnostic.jsonl");
        let mut sink = Sink::new(&path).unwrap();
        let payload = "x".repeat(8192);
        for _ in 0..200 {
            sink.write("sample", json!({"bounded":payload}), false);
        }
        sink.finish(1);
        drop(sink);
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() <= LIMIT);
        let records = std::str::from_utf8(&bytes)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        let finality = records.last().unwrap();
        assert_eq!(finality["kind"], "finish");
        assert_eq!(finality["data"]["capture"], "partial");
        assert_eq!(finality["data"]["exit_code"], 1);
        assert!(finality["data"]["dropped"].as_u64().unwrap() > 0);
        assert!(Sink::new(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn containment() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("diagnostic.jsonl");
        let mut sink = Sink::new(&path).unwrap();
        let (parent, start) = sink.enter("guard");
        let (child, inside) = sink.enter("cargo.test");
        sink.leave(child, inside, "failed");
        sink.leave(parent, start, "incomplete");
        sink.finish(1);
        drop(sink);
        let text = std::fs::read_to_string(path).unwrap();
        let records = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records[1]["data"]["parent"], parent);
        let ended = records[2]["data"]["end_offset_ns"].as_u64().unwrap();
        assert!(inside >= start);
        assert!(ended >= inside);
        assert_eq!(records[2]["data"]["duration_ns"], ended - inside);
        assert!(records[3]["data"]["end_offset_ns"].as_u64().unwrap() >= ended);
        assert_eq!(records[4]["data"]["capture"], "partial");
    }

    #[test]
    fn unwritable() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("diagnostic.jsonl");
        let mut sink = Sink::new(&path).unwrap();
        sink.file = File::open(&path).unwrap();
        sink.write("start", json!({}), false);
        assert!(sink.failed);
        sink.finish(1);
        assert!(std::fs::read(path).unwrap().is_empty());
    }
}
