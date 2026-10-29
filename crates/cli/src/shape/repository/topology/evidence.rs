use super::registry::{fetch, qualified, url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub(crate) struct Evidence {
    pub schema: &'static str,
    pub input: String,
    pub indices: BTreeMap<String, String>,
    pub versions: usize,
    pub edges: usize,
    pub complete: bool,
}

#[derive(Deserialize, Serialize)]
struct Stored {
    format: u32,
    name: String,
    url: String,
    sha256: String,
}

pub(super) trait Source {
    fn prepare(&mut self, names: &BTreeSet<String>);
    fn read(&mut self, name: &str) -> Result<Vec<u8>, String>;
    fn confirm(&self) -> Result<(), String>;
    fn indices(&self) -> &BTreeMap<String, String>;
}

type Materials = BTreeMap<String, Result<Vec<u8>, String>>;

pub(super) struct Reader {
    home: PathBuf,
    pub indices: BTreeMap<String, String>,
    started: Instant,
    budget: AtomicUsize,
    agent: ureq::Agent,
    prepared: Materials,
}

impl Reader {
    pub fn new() -> Result<Self, String> {
        let home = plumb::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| plumb::config::data("plumb"))
            .ok_or("topology has no managed data home")?;
        Ok(Self {
            home: home.join("cache/topology/v1"),
            indices: BTreeMap::new(),
            started: Instant::now(),
            budget: AtomicUsize::new(0),
            prepared: BTreeMap::new(),
            agent: ureq::Agent::config_builder()
                .https_only(true)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(15)))
                .build()
                .into(),
        })
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, String> {
        qualified(name)?;
        if self.started.elapsed() >= Duration::from_secs(180) {
            return Err("declaration index time budget exhausted".into());
        }
        let url = url(name);
        let pointer = self.home.join(format!("{name}.json"));
        let cached = pointer.exists();
        let bytes = if cached {
            let stored: Stored = serde_json::from_slice(&bounded(&pointer, 65536)?)
                .map_err(|error| error.to_string())?;
            let identity = (stored.format, stored.name.as_str(), stored.url.as_str());
            if identity != (1, name, url.as_str()) || !checksum(&stored.sha256) {
                return Err(format!("unqualified declaration cache for {name}"));
            }
            let bytes = bounded(
                &self.home.join(format!("{}.index", stored.sha256)),
                16 * 1024 * 1024,
            )?;
            if digest(&bytes) != stored.sha256 {
                return Err(format!("declaration cache checksum differs for {name}"));
            }
            bytes
        } else {
            let bytes = self
                .prepared
                .remove(name)
                .unwrap_or_else(|| self.download(name))?;
            super::registry::Entry::read(name, &bytes)?;
            fs::create_dir_all(&self.home).map_err(|error| error.to_string())?;
            let hash = digest(&bytes);
            let object = self.home.join(format!("{hash}.index"));
            if object.exists() {
                if bounded(&object, 16 * 1024 * 1024)? != bytes {
                    return Err("declaration cache object differs".into());
                }
            } else {
                fs::write(object, &bytes).map_err(|error| error.to_string())?;
            }
            let stored = Stored {
                format: 1,
                name: name.into(),
                url,
                sha256: hash,
            };
            plumb::seat::replace::write(
                &pointer,
                &serde_json::to_vec(&stored).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            bytes
        };
        if cached {
            self.budget.fetch_add(bytes.len(), Ordering::Relaxed);
        }
        if self.budget.load(Ordering::Relaxed) > 256 * 1024 * 1024 {
            return Err("declaration index byte budget exhausted".into());
        }
        self.indices.insert(name.into(), digest(&bytes));
        Ok(bytes)
    }

    pub fn confirm(&self) -> Result<(), String> {
        for hash in self.indices.values() {
            let bytes = bounded(&self.home.join(format!("{hash}.index")), 16 * 1024 * 1024)?;
            if digest(&bytes) != *hash {
                return Err("declaration index changed during capture".into());
            }
        }
        Ok(())
    }
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn checksum(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl Source for Reader {
    fn prepare(&mut self, names: &BTreeSet<String>) {
        let pending = Mutex::new(
            names
                .iter()
                .filter(|name| !self.home.join(format!("{name}.json")).exists())
                .cloned()
                .collect::<VecDeque<_>>(),
        );
        let results = Mutex::new(BTreeMap::new());
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| self.batch(&pending, &results));
            }
        });
        self.prepared
            .extend(results.into_inner().expect("prepared indices"));
    }
    fn read(&mut self, name: &str) -> Result<Vec<u8>, String> {
        Reader::read(self, name)
    }
    fn confirm(&self) -> Result<(), String> {
        Reader::confirm(self)
    }
    fn indices(&self) -> &BTreeMap<String, String> {
        &self.indices
    }
}

fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("declaration cache exceeds byte budget".into());
    }
    Ok(bytes)
}

impl Reader {
    fn batch(&self, pending: &Mutex<VecDeque<String>>, results: &Mutex<Materials>) {
        loop {
            let Some(name) = pending.lock().expect("index queue").pop_front() else {
                break;
            };
            let result = self.download(&name);
            results.lock().expect("index results").insert(name, result);
        }
    }

    fn download(&self, name: &str) -> Result<Vec<u8>, String> {
        qualified(name)?;
        if self.started.elapsed() >= Duration::from_secs(180) {
            return Err("declaration index time budget exhausted".into());
        }
        if self.budget.load(Ordering::Relaxed) >= 256 * 1024 * 1024 {
            return Err("declaration index byte budget exhausted".into());
        }
        let body = fetch(&self.agent, &url(name))?;
        let total = self.budget.fetch_add(body.len(), Ordering::Relaxed) + body.len();
        if total > 256 * 1024 * 1024 {
            return Err("declaration index byte budget exhausted".into());
        }
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt() {
        let root = tempfile::tempdir().unwrap();
        let mut reader = Reader::new().unwrap();
        reader.home = root.path().into();
        let stored = Stored {
            format: 1,
            name: "bridge".into(),
            url: url("bridge"),
            sha256: digest(b"original"),
        };
        fs::write(
            root.path().join(format!("{}.index", stored.sha256)),
            b"corrupt",
        )
        .unwrap();
        fs::write(
            root.path().join("bridge.json"),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();
        assert!(reader.read("bridge").unwrap_err().contains("checksum"));
        assert!(reader.indices.is_empty());
    }
}
