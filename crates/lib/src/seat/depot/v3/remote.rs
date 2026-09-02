use super::{Kind, Manifest, Pointer, Route};

const ATTEMPTS: usize = 3;

pub struct Query<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub kind: Kind,
}

#[derive(Clone, Debug)]
pub struct Generation {
    pub pointer: Pointer,
    pub manifest: Manifest,
    base: String,
}

impl Generation {
    pub fn latest(query: Query<'_>) -> Result<Option<Self>, String> {
        let source = query.source.trim_end_matches('/');
        let route = Route::new(query.channel, query.kind, query.version);
        let key = super::latest(route)?;
        let Some(bytes) = pull(&format!("{source}/{key}"))? else {
            return Ok(None);
        };
        let pointer = Pointer::parse(&bytes)?;
        let standing = (
            pointer.product.as_str(),
            pointer.channel.as_str(),
            pointer.version.as_str(),
            pointer.kind,
        );
        let wanted = (query.product, query.channel, query.version, query.kind);
        if standing != wanted {
            return Ok(None);
        }
        Self::exact(source, pointer).map(Some)
    }

    pub fn exact(source: &str, pointer: Pointer) -> Result<Self, String> {
        let source = source.trim_end_matches('/');
        let route = Route::new(&pointer.channel, pointer.kind, &pointer.version);
        let expected = super::manifest(source, route, &pointer.generation)?;
        if pointer.manifest.url != expected {
            return Err("depot pointer names another source".into());
        }
        let bytes = pull(&pointer.manifest.url)?
            .ok_or_else(|| format!("depot generation {} has no manifest", pointer.generation))?;
        let manifest = Manifest::parse(&bytes)?;
        pointer.bind(&manifest, &bytes)?;
        let base = pointer
            .manifest
            .url
            .strip_suffix(super::LEAF)
            .expect("a valid depot manifest URL ends in its leaf")
            .to_string();
        Ok(Self {
            pointer,
            manifest,
            base,
        })
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let bytes = pull(&format!("{}objects/{path}", self.base))?
            .ok_or_else(|| format!("depot object {path} is absent"))?;
        let executable = self
            .manifest
            .objects
            .iter()
            .find(|held| held.path == path)
            .map(|held| held.executable)
            .ok_or_else(|| format!("depot manifest names no object at {path}"))?;
        self.manifest.verify(path, &bytes, executable)?;
        Ok(bytes)
    }

    pub fn url(&self) -> &str {
        &self.base
    }
}

fn pull(url: &str) -> Result<Option<Vec<u8>>, String> {
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        match draw(url) {
            Ok(bytes) => return Ok(bytes),
            Err(error) => last = error,
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(std::time::Duration::from_millis(200 * (attempt as u64 + 1)));
        }
    }
    Err(last)
}

fn draw(url: &str) -> Result<Option<Vec<u8>>, String> {
    let response = match ureq::get(url).call() {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(format!("cannot fetch {url}: {error}")),
    };
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut body.as_reader(), &mut bytes)
        .map_err(|error| format!("cannot read {url}: {error}"))?;
    Ok(Some(bytes))
}
